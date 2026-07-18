//! The kernel: the single place agent actions are authorized and executed.
//!
//! [`Kernel::authorize_and_execute`] is the chokepoint. It records the action,
//! runs the deterministic policy, and then either denies, executes, or persists
//! an approval request and *awaits a real human decision* on an async channel
//! (no busy-loop). Emergency stop cancels waiting approvals and aborts in-flight
//! execution.

use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::time::Duration;

use sqlx::SqlitePool;
use tokio::sync::{oneshot, watch};

use crate::action::ActionRequest;
use crate::approval::{ApprovalStatus, ApprovalStore};
use crate::error::KernelResult;
use crate::events::{KernelEvent, KernelEventKind, TraceSink};
use crate::policy::{self, PolicyConfig, PolicyDecision, KERNEL_POLICY_VERSION};

/// How a pending approval was resolved.
#[derive(Debug, Clone)]
pub enum Resolution {
    Approved { resolver: Option<String> },
    Rejected { resolver: Option<String> },
    Cancelled { reason: String },
}

/// The result of taking an action through the kernel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionOutcome {
    /// The action ran; `output` is the tool result.
    Executed { output: String },
    /// Policy denied the action; it never reached the executor.
    Denied { code: String, reason: String },
    /// A human rejected the action.
    Rejected { reason: String },
    /// The wait was cancelled (emergency stop / run cancelled / UI closed).
    Cancelled { reason: String },
    /// The approval expired before a human responded.
    Expired,
    /// Execution exceeded its timeout and was aborted.
    TimedOut,
    /// The executor itself returned an error.
    Failed { error: String },
    /// Emergency stop fired before/while executing.
    EmergencyStopped,
}

impl ExecutionOutcome {
    /// True only when the tool actually executed.
    pub fn executed(&self) -> bool {
        matches!(self, ExecutionOutcome::Executed { .. })
    }

    /// A structured JSON message describing this outcome, suitable for feeding
    /// back to the agent so it can react to a denial/rejection.
    pub fn agent_response(&self) -> String {
        let v = match self {
            ExecutionOutcome::Executed { output } => {
                serde_json::json!({ "kernel_decision": "executed", "output": output })
            }
            ExecutionOutcome::Denied { code, reason } => {
                serde_json::json!({ "kernel_decision": "denied", "code": code, "reason": reason })
            }
            ExecutionOutcome::Rejected { reason } => {
                serde_json::json!({ "kernel_decision": "rejected", "reason": reason })
            }
            ExecutionOutcome::Cancelled { reason } => {
                serde_json::json!({ "kernel_decision": "cancelled", "reason": reason })
            }
            ExecutionOutcome::Expired => {
                serde_json::json!({ "kernel_decision": "expired", "reason": "Approval expired before a human responded." })
            }
            ExecutionOutcome::TimedOut => {
                serde_json::json!({ "kernel_decision": "timed_out", "reason": "Action exceeded its timeout." })
            }
            ExecutionOutcome::Failed { error } => {
                serde_json::json!({ "kernel_decision": "failed", "error": error })
            }
            ExecutionOutcome::EmergencyStopped => {
                serde_json::json!({ "kernel_decision": "emergency_stopped", "reason": "Emergency stop is active." })
            }
        };
        v.to_string()
    }
}

struct RunRuntime {
    action_count: u64,
    emergency_tx: watch::Sender<bool>,
    pending_approvals: HashSet<String>,
}

impl RunRuntime {
    fn new() -> Self {
        let (emergency_tx, _rx) = watch::channel(false);
        Self {
            action_count: 0,
            emergency_tx,
            pending_approvals: HashSet::new(),
        }
    }
}

struct Inner {
    runs: HashMap<i64, RunRuntime>,
    waiters: HashMap<String, oneshot::Sender<Resolution>>,
}

/// The kernel. One instance per application; cheap to share behind an `Arc`.
pub struct Kernel {
    store: ApprovalStore,
    inner: std::sync::Mutex<Inner>,
    /// How long to wait for a human before an approval expires. `None` = wait
    /// indefinitely (until resolved / cancelled).
    approval_timeout: Option<Duration>,
}

impl Kernel {
    pub fn new(pool: SqlitePool, approval_timeout: Option<Duration>) -> Self {
        Self {
            store: ApprovalStore::new(pool),
            inner: std::sync::Mutex::new(Inner {
                runs: HashMap::new(),
                waiters: HashMap::new(),
            }),
            approval_timeout,
        }
    }

    pub fn store(&self) -> &ApprovalStore {
        &self.store
    }

    pub fn current_action_count(&self, run_id: i64) -> u64 {
        let inner = self.inner.lock().unwrap();
        inner.runs.get(&run_id).map(|r| r.action_count).unwrap_or(0)
    }

    /// Ensure a run runtime exists; return (emergency receiver, action_count,
    /// emergency_now).
    fn ensure_run(&self, run_id: i64) -> (watch::Receiver<bool>, u64, bool) {
        let mut inner = self.inner.lock().unwrap();
        let rt = inner.runs.entry(run_id).or_insert_with(RunRuntime::new);
        let rx = rt.emergency_tx.subscribe();
        let count = rt.action_count;
        let emergency_now = *rt.emergency_tx.borrow();
        (rx, count, emergency_now)
    }

    fn increment_action_count(&self, run_id: i64) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(rt) = inner.runs.get_mut(&run_id) {
            rt.action_count += 1;
        }
    }

    /// Resolve a pending approval (called by the UI / HTTP layer). Idempotent:
    /// the first call wins; a duplicate call returns `false`.
    pub fn resolve_approval(&self, approval_id: &str, resolution: Resolution) -> bool {
        let mut inner = self.inner.lock().unwrap();
        match inner.waiters.remove(approval_id) {
            Some(tx) => {
                for rt in inner.runs.values_mut() {
                    rt.pending_approvals.remove(approval_id);
                }
                tx.send(resolution).is_ok()
            }
            None => false,
        }
    }

    /// Emergency stop for a run: block new actions, cancel every waiting
    /// approval, and signal any in-flight execution to abort. Returns the
    /// approval ids that were cancelled so the caller can mark the ledger.
    pub async fn emergency_stop(&self, run_id: i64) -> KernelResult<Vec<String>> {
        let ids: Vec<String> = {
            let mut inner = self.inner.lock().unwrap();
            let rt = inner.runs.entry(run_id).or_insert_with(RunRuntime::new);
            // send_replace updates the retained value even with no live receivers
            // (plain `send` fails and leaves the flag unset in that case).
            rt.emergency_tx.send_replace(true);
            let ids: Vec<String> = rt.pending_approvals.drain().collect();
            for id in &ids {
                if let Some(tx) = inner.waiters.remove(id) {
                    let _ = tx.send(Resolution::Cancelled {
                        reason: "emergency_stop".into(),
                    });
                }
            }
            ids
        };
        for id in &ids {
            let _ = self
                .store
                .mark_resolved(
                    id,
                    ApprovalStatus::Cancelled,
                    Some("system:emergency_stop"),
                    Some("emergency_stop"),
                )
                .await;
        }
        Ok(ids)
    }

    /// Global panic button: emergency-stop every known run. Returns all cancelled
    /// approval ids.
    pub async fn emergency_stop_all(&self) -> KernelResult<Vec<String>> {
        let run_ids: Vec<i64> = {
            let inner = self.inner.lock().unwrap();
            inner.runs.keys().copied().collect()
        };
        let mut all = Vec::new();
        for rid in run_ids {
            all.extend(self.emergency_stop(rid).await?);
        }
        Ok(all)
    }

    /// Clear the emergency-stop flag for a run (e.g. when a new supervised
    /// session is activated).
    pub fn clear_emergency(&self, run_id: i64) {
        let mut inner = self.inner.lock().unwrap();
        if let Some(rt) = inner.runs.get_mut(&run_id) {
            rt.emergency_tx.send_replace(false);
        }
    }

    /// The single chokepoint. Every agent action must pass through here.
    ///
    /// `exec` is only ever invoked with the immutable `req` that was evaluated
    /// and (if applicable) approved — never a regenerated action.
    pub async fn authorize_and_execute<F, Fut>(
        &self,
        mut cfg: PolicyConfig,
        req: ActionRequest,
        sink: &dyn TraceSink,
        exec: F,
    ) -> ExecutionOutcome
    where
        F: FnOnce(ActionRequest) -> Fut,
        Fut: Future<Output = Result<String, String>> + Send,
    {
        let run_id = req.run_id;
        let (emergency_rx, count, emergency_now) = self.ensure_run(run_id);
        cfg.action_count = count;
        if emergency_now {
            cfg.emergency_stopped = true;
        }

        sink.record(KernelEvent::new(
            run_id,
            KernelEventKind::ActionRequested,
            &req.id,
            &req.tool_name,
            format!(
                "Action requested: {} ({})",
                req.tool_name,
                req.kind.as_str()
            ),
        ))
        .await;

        // 1. Deterministic policy decision.
        match policy::evaluate(&cfg, &req) {
            PolicyDecision::Deny { code, reason } => {
                sink.record(
                    KernelEvent::new(
                        run_id,
                        KernelEventKind::PolicyDenied,
                        &req.id,
                        &req.tool_name,
                        format!("Policy denied: {}", reason),
                    )
                    .with_details(
                        serde_json::json!({ "code": code, "reason": reason }).to_string(),
                    ),
                )
                .await;
                return ExecutionOutcome::Denied { code, reason };
            }
            PolicyDecision::RequireApproval {
                reason,
                required_role,
            } => {
                if let Err(outcome) = self
                    .await_human_decision(&req, &cfg, &reason, required_role.as_deref(), sink)
                    .await
                {
                    return outcome;
                }
                // Approved -> fall through to execution.
            }
            PolicyDecision::Allow => {
                sink.record(KernelEvent::new(
                    run_id,
                    KernelEventKind::PolicyAllowed,
                    &req.id,
                    &req.tool_name,
                    "Policy allowed action.",
                ))
                .await;
            }
        }

        // 2. Execute the immutable action, guarded by emergency stop + timeout.
        self.execute_guarded(&cfg, req, sink, emergency_rx, exec)
            .await
    }

    /// Persist an approval, register a waiter, and await a real human decision.
    /// Returns `Ok(())` on approval; `Err(outcome)` for any terminal non-execute
    /// result (already recorded to the sink and ledger).
    async fn await_human_decision(
        &self,
        req: &ActionRequest,
        cfg: &PolicyConfig,
        reason: &str,
        required_role: Option<&str>,
        sink: &dyn TraceSink,
    ) -> Result<(), ExecutionOutcome> {
        let run_id = req.run_id;

        let approval_id = match self
            .store
            .create_pending(
                req,
                reason,
                required_role,
                KERNEL_POLICY_VERSION,
                &cfg.snapshot(),
            )
            .await
        {
            Ok(id) => id,
            Err(e) => {
                return Err(ExecutionOutcome::Failed {
                    error: format!("failed to persist approval: {}", e),
                })
            }
        };

        let (tx, rx) = oneshot::channel::<Resolution>();
        {
            let mut inner = self.inner.lock().unwrap();
            inner.waiters.insert(approval_id.clone(), tx);
            let rt = inner.runs.entry(run_id).or_insert_with(RunRuntime::new);
            rt.pending_approvals.insert(approval_id.clone());
        }

        sink.record(
            KernelEvent::new(
                run_id,
                KernelEventKind::ApprovalRequested,
                &req.id,
                &req.tool_name,
                format!("Approval requested: {}", reason),
            )
            .with_details(serde_json::json!({ "approval_id": approval_id }).to_string()),
        )
        .await;

        // Await the human. `.await` yields — no busy-loop, no runtime blocking.
        let resolution = match self.approval_timeout {
            Some(d) => match tokio::time::timeout(d, rx).await {
                Ok(Ok(r)) => r,
                Ok(Err(_)) => Resolution::Cancelled {
                    reason: "waiter dropped".into(),
                },
                Err(_) => Resolution::Cancelled {
                    reason: "expired".into(),
                },
            },
            None => match rx.await {
                Ok(r) => r,
                Err(_) => Resolution::Cancelled {
                    reason: "waiter dropped".into(),
                },
            },
        };

        // Deregister the waiter (safe even if already removed by a resolver).
        {
            let mut inner = self.inner.lock().unwrap();
            inner.waiters.remove(&approval_id);
            if let Some(rt) = inner.runs.get_mut(&run_id) {
                rt.pending_approvals.remove(&approval_id);
            }
        }

        match resolution {
            Resolution::Approved { resolver } => {
                let _ = self
                    .store
                    .mark_resolved(
                        &approval_id,
                        ApprovalStatus::Approved,
                        resolver.as_deref(),
                        Some("approved"),
                    )
                    .await;
                sink.record(KernelEvent::new(
                    run_id,
                    KernelEventKind::ApprovalApproved,
                    &req.id,
                    &req.tool_name,
                    "Human approved the action.",
                ))
                .await;
                Ok(())
            }
            Resolution::Rejected { resolver } => {
                let _ = self
                    .store
                    .mark_resolved(
                        &approval_id,
                        ApprovalStatus::Rejected,
                        resolver.as_deref(),
                        Some("rejected"),
                    )
                    .await;
                sink.record(KernelEvent::new(
                    run_id,
                    KernelEventKind::ApprovalRejected,
                    &req.id,
                    &req.tool_name,
                    "Human rejected the action.",
                ))
                .await;
                Err(ExecutionOutcome::Rejected {
                    reason: "Rejected by human reviewer.".into(),
                })
            }
            Resolution::Cancelled { reason } => {
                let (status, expired) = if reason == "expired" {
                    (ApprovalStatus::Expired, true)
                } else {
                    (ApprovalStatus::Cancelled, false)
                };
                let _ = self
                    .store
                    .mark_resolved(&approval_id, status, Some("system"), Some(&reason))
                    .await;
                sink.record(
                    KernelEvent::new(
                        run_id,
                        KernelEventKind::ApprovalCancelled,
                        &req.id,
                        &req.tool_name,
                        format!("Approval not granted ({}).", reason),
                    )
                    .with_details(serde_json::json!({ "reason": reason }).to_string()),
                )
                .await;
                Err(if expired {
                    ExecutionOutcome::Expired
                } else {
                    ExecutionOutcome::Cancelled { reason }
                })
            }
        }
    }

    async fn execute_guarded<F, Fut>(
        &self,
        cfg: &PolicyConfig,
        req: ActionRequest,
        sink: &dyn TraceSink,
        mut emergency_rx: watch::Receiver<bool>,
        exec: F,
    ) -> ExecutionOutcome
    where
        F: FnOnce(ActionRequest) -> Fut,
        Fut: Future<Output = Result<String, String>> + Send,
    {
        let run_id = req.run_id;

        // Defense in depth: re-check emergency immediately before running.
        if *emergency_rx.borrow() {
            sink.record(KernelEvent::new(
                run_id,
                KernelEventKind::EmergencyStop,
                &req.id,
                &req.tool_name,
                "Emergency stop active; action aborted before execution.",
            ))
            .await;
            return ExecutionOutcome::EmergencyStopped;
        }

        sink.record(KernelEvent::new(
            run_id,
            KernelEventKind::ActionStarted,
            &req.id,
            &req.tool_name,
            "Action execution started.",
        ))
        .await;

        let effective_ms = cfg.effective_timeout_ms(req.timeout_ms);
        let action_id = req.id.clone();
        let tool_name = req.tool_name.clone();

        // Execute the immutable request. `exec` receives a clone of the SAME
        // request that was evaluated/approved — never a regenerated one.
        let exec_fut = exec(req);
        let timed = async {
            if effective_ms == 0 {
                ExecStep::Done(exec_fut.await)
            } else {
                match tokio::time::timeout(Duration::from_millis(effective_ms), exec_fut).await {
                    Ok(r) => ExecStep::Done(r),
                    Err(_) => ExecStep::Timeout,
                }
            }
        };

        let outcome = tokio::select! {
            biased;
            _ = wait_emergency(&mut emergency_rx) => ExecStep::Emergency,
            step = timed => step,
        };

        match outcome {
            ExecStep::Done(Ok(output)) => {
                self.increment_action_count(run_id);
                sink.record(
                    KernelEvent::new(
                        run_id,
                        KernelEventKind::ActionCompleted,
                        &action_id,
                        &tool_name,
                        "Action completed successfully.",
                    )
                    .with_details(
                        serde_json::json!({
                            "action_count": self.current_action_count(run_id)
                        })
                        .to_string(),
                    ),
                )
                .await;
                ExecutionOutcome::Executed { output }
            }
            ExecStep::Done(Err(error)) => {
                sink.record(KernelEvent::new(
                    run_id,
                    KernelEventKind::ActionFailed,
                    &action_id,
                    &tool_name,
                    format!("Action failed: {}", error),
                ))
                .await;
                ExecutionOutcome::Failed { error }
            }
            ExecStep::Timeout => {
                sink.record(KernelEvent::new(
                    run_id,
                    KernelEventKind::ActionTimedOut,
                    &action_id,
                    &tool_name,
                    format!("Action timed out after {} ms.", effective_ms),
                ))
                .await;
                ExecutionOutcome::TimedOut
            }
            ExecStep::Emergency => {
                sink.record(KernelEvent::new(
                    run_id,
                    KernelEventKind::EmergencyStop,
                    &action_id,
                    &tool_name,
                    "Emergency stop fired during execution; action aborted.",
                ))
                .await;
                ExecutionOutcome::EmergencyStopped
            }
        }
    }

    /// Restart recovery: cancel any approvals left `pending` by a previous
    /// process (their async waiters did not survive the restart).
    pub async fn recover_pending_on_startup(&self) -> KernelResult<u64> {
        self.store
            .cancel_stale_pending("cancelled on application restart")
            .await
    }
}

enum ExecStep {
    Done(Result<String, String>),
    Timeout,
    Emergency,
}

/// Resolve when the emergency flag transitions to `true`.
async fn wait_emergency(rx: &mut watch::Receiver<bool>) {
    loop {
        if *rx.borrow() {
            return;
        }
        if rx.changed().await.is_err() {
            // Sender dropped: emergency can never fire on this receiver.
            std::future::pending::<()>().await;
        }
    }
}
