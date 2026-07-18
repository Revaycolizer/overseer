//! Bridge between the Overseer engine and the deterministic `overseer-kernel`.
//!
//! [`kernel_execute`] is the ONE function every side-effecting agent action goes
//! through (native tool, Python tool, shell command, computer control). It builds
//! a typed [`ActionRequest`], evaluates it against policy, and — on allow or human
//! approval — runs the immutable action, recording the full lifecycle to the
//! execution trace. There is no other authorized path to `call_dynamic_tool`.

use std::sync::OnceLock;
use std::time::Duration;

use async_trait::async_trait;
use serde_json::Value;
use sqlx::SqlitePool;

use overseer_kernel::{
    ActionKind, ActionRequest, ExecutionOutcome, Kernel, KernelEvent, PolicyConfig, PolicyDecision,
    Resolution, TraceSink,
};

use crate::db::Database;
use crate::engine::utils::discovery::ToolInfo;

/// Process-wide kernel. Overseer runs a single local database, so one kernel
/// instance owns all approval coordination and the emergency-stop state.
static KERNEL: OnceLock<Kernel> = OnceLock::new();

/// Default time a run will wait for a human approval before it expires (ms).
const DEFAULT_APPROVAL_WAIT_MS: u64 = 300_000;

/// Get (initializing on first use) the global kernel bound to `pool`.
pub fn global_kernel(pool: &SqlitePool) -> &'static Kernel {
    KERNEL.get_or_init(|| {
        Kernel::new(
            pool.clone(),
            Some(Duration::from_millis(DEFAULT_APPROVAL_WAIT_MS)),
        )
    })
}

/// Read-only OS verbs that never mutate the machine and so are treated as
/// observations (allowed without approval, not counted against the action cap).
fn is_observation_verb(action: &str) -> bool {
    matches!(
        action,
        "get_state" | "state" | "observe" | "inspect" | "snapshot" | "status" | "list_processes"
    )
}

/// Classify a tool invocation into a typed [`ActionKind`].
pub fn classify_action_kind(tool: &ToolInfo, domain: &str) -> ActionKind {
    if tool.language.eq_ignore_ascii_case("python") {
        ActionKind::PythonTool
    } else if tool.name == "os_tool" || matches!(domain, "computer" | "desktop") {
        ActionKind::ComputerControl
    } else if tool.name == "code_executor" || tool.name == "shell" {
        ActionKind::ShellCommand
    } else {
        ActionKind::NativeTool
    }
}

/// A [`TraceSink`] that records kernel events into the `execution_events` table.
/// Events are only written when there is a real execution run to attach them to.
struct DbTraceSink<'a> {
    db: &'a Database,
    run_id: Option<i64>,
    domain: String,
    step: String,
}

#[async_trait]
impl<'a> TraceSink for DbTraceSink<'a> {
    async fn record(&self, event: KernelEvent) {
        let Some(run_id) = self.run_id else {
            return;
        };
        let seq: i64 = sqlx::query_scalar::<_, i64>(
            "SELECT COALESCE(MAX(sequence), -1) + 1 FROM execution_events WHERE run_id = ?",
        )
        .bind(run_id)
        .fetch_one(&self.db.pool)
        .await
        .unwrap_or(0);

        let _ = crate::backend::log_execution_event(
            self.db,
            run_id,
            seq,
            event.kind.as_str(),
            event.kind.status(),
            Some(&self.domain),
            Some(&self.step),
            Some(&event.tool_name),
            None,
            Some(&event.message),
            event.details.as_deref(),
        )
        .await;
    }
}

/// Conservative default used only if supervision state cannot be read.
fn default_supervision_state() -> crate::backend::ComputerSupervisionState {
    crate::backend::ComputerSupervisionState {
        supervision_mode: "approval_required".to_string(),
        session_state: "idle".to_string(),
        session_note: String::new(),
        allowlist: Vec::new(),
        max_actions: 25,
        action_count: 0,
        timeout_ms: 600_000,
        approval_state: "idle".to_string(),
        pending_tool: String::new(),
        pending_action: String::new(),
        pending_domain: String::new(),
        session_started_at: None,
    }
}

/// Build the deterministic policy config for one action from the persisted
/// computer-supervision settings and the agent's granted capabilities.
fn build_policy_config(
    state: &crate::backend::ComputerSupervisionState,
    capabilities: &[String],
) -> PolicyConfig {
    PolicyConfig {
        capability_set: capabilities.iter().cloned().collect(),
        allowlist: state.allowlist.iter().cloned().collect(),
        denylist: Default::default(),
        max_actions: state.max_actions.max(0) as u64,
        action_count: 0, // injected by the kernel from its per-run counter
        timeout_ms_cap: state.timeout_ms.max(0) as u64,
        emergency_stopped: state.session_state == "emergency_stopped",
        require_approval_kinds: [ActionKind::ComputerControl, ActionKind::ShellCommand]
            .into_iter()
            .collect(),
        approval_required_default: false,
        observation_tools: ["vision_analysis"].iter().map(|s| s.to_string()).collect(),
        escalate_high_risk: true,
    }
}

/// THE chokepoint. Authorize and (if permitted) execute one tool invocation.
///
/// `capabilities` is the set of tool names the agent is allowed to use. On any
/// non-execute decision the returned [`ExecutionOutcome`] carries a structured
/// message (`outcome.agent_response()`) suitable for feeding back to the agent.
pub async fn kernel_execute(
    db: &Database,
    run_id: Option<i64>,
    domain: &str,
    step: &str,
    tool: &ToolInfo,
    arguments: Value,
    capabilities: &[String],
) -> ExecutionOutcome {
    // The production executor: the real dynamic-tool dispatcher. `kernel_execute`
    // is the only place it is wired; the injectable variant below is crate-private
    // and used only by integration tests with a recording/safe executor.
    let tool_owned = tool.clone();
    kernel_execute_with(
        db,
        run_id,
        domain,
        step,
        tool,
        arguments,
        capabilities,
        move |req: ActionRequest| async move {
            crate::engine::tools::call_dynamic_tool(&tool_owned, req.arguments)
                .await
                .map_err(|e| e.to_string())
        },
    )
    .await
}

/// Same governed pipeline as [`kernel_execute`] but with an injectable executor.
/// Crate-private so it cannot be used to bypass policy from outside the crate;
/// integration tests pass a recording/safe executor to exercise the bridge
/// without a real desktop mutation.
#[allow(clippy::too_many_arguments)]
pub(crate) async fn kernel_execute_with<F, Fut>(
    db: &Database,
    run_id: Option<i64>,
    domain: &str,
    step: &str,
    tool: &ToolInfo,
    arguments: Value,
    capabilities: &[String],
    exec: F,
) -> ExecutionOutcome
where
    F: FnOnce(ActionRequest) -> Fut,
    Fut: std::future::Future<Output = Result<String, String>> + Send,
{
    let pool = db.pool.clone();
    let kernel = global_kernel(&pool);

    // On any error reading supervision state, fall back to the most restrictive
    // sensible default (approval_required) rather than failing open.
    let state = crate::backend::get_computer_supervision_state(db)
        .await
        .unwrap_or_else(|_| default_supervision_state());
    let tool_timeout_ms =
        crate::backend::get_setting_i64(db, "tool_timeout_ms", 45_000).await as u64;
    let mut cfg = build_policy_config(&state, capabilities);

    let kind = classify_action_kind(tool, domain);

    // Observation OS verbs are read-only: mark this specific tool as an
    // observation so it is neither counted nor sent for approval.
    if kind == ActionKind::ComputerControl {
        if let Some(action) = arguments.get("action").and_then(|v| v.as_str()) {
            if is_observation_verb(action) {
                cfg.observation_tools.insert(tool.name.clone());
            }
        }
    }
    // In approval_required mode every non-observation action still needs
    // approval; the deterministic policy makes the final decision.
    if state.supervision_mode == "approval_required" {
        cfg.approval_required_default = true;
    }

    let summary = crate::engine::orchestrator::dispatch::tools::summarize_computer_action(
        &tool.name, &arguments,
    );
    let req = ActionRequest::new(
        run_id.unwrap_or(0),
        kind,
        &tool.name,
        arguments,
        vec![tool.name.clone()],
        tool_timeout_ms,
        summary,
    );

    let sink = DbTraceSink {
        db,
        run_id,
        domain: domain.to_string(),
        step: step.to_string(),
    };

    kernel.authorize_and_execute(cfg, req, &sink, exec).await
}

/// Non-blocking policy check for contexts that cannot wait for a human (e.g. the
/// voice loop). Returns the deterministic decision; callers must only proceed on
/// [`PolicyDecision::Allow`]. This is still the kernel's policy — there is no
/// separate rule set.
pub async fn evaluate_named(
    db: &Database,
    domain: &str,
    kind: ActionKind,
    tool_name: &str,
    arguments: &Value,
    capabilities: &[String],
) -> PolicyDecision {
    let state = crate::backend::get_computer_supervision_state(db)
        .await
        .unwrap_or_else(|_| default_supervision_state());
    let mut cfg = build_policy_config(&state, capabilities);
    if kind == ActionKind::ComputerControl {
        if let Some(action) = arguments.get("action").and_then(|v| v.as_str()) {
            if is_observation_verb(action) {
                cfg.observation_tools.insert(tool_name.to_string());
            }
        }
    }
    if state.supervision_mode == "approval_required" {
        cfg.approval_required_default = true;
    }
    let req = ActionRequest::new(
        0,
        kind,
        tool_name,
        arguments.clone(),
        vec![tool_name.to_string()],
        0,
        String::new(),
    );
    overseer_kernel::evaluate(&cfg, &req)
}

// ---------- resolution / emergency-stop entry points (called by UI + HTTP) ----------

/// Approve the given pending approval. Returns true if this call resolved it.
pub fn approve(db: &Database, approval_id: &str, resolver: &str) -> bool {
    let kernel = global_kernel(&db.pool);
    kernel.resolve_approval(
        approval_id,
        Resolution::Approved {
            resolver: Some(resolver.to_string()),
        },
    )
}

/// Reject the given pending approval. Returns true if this call resolved it.
pub fn reject(db: &Database, approval_id: &str, resolver: &str) -> bool {
    let kernel = global_kernel(&db.pool);
    kernel.resolve_approval(
        approval_id,
        Resolution::Rejected {
            resolver: Some(resolver.to_string()),
        },
    )
}

/// Emergency stop for a run: block new actions, cancel any waiting approval, and
/// abort in-flight execution. Also records an `emergency_stop` trace event.
pub async fn emergency_stop(db: &Database, run_id: i64) {
    let kernel = global_kernel(&db.pool);
    let cancelled = kernel.emergency_stop(run_id).await.unwrap_or_default();
    let seq: i64 = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(sequence), -1) + 1 FROM execution_events WHERE run_id = ?",
    )
    .bind(run_id)
    .fetch_one(&db.pool)
    .await
    .unwrap_or(0);
    let details = serde_json::json!({ "cancelled_approvals": cancelled }).to_string();
    let _ = crate::backend::log_execution_event(
        db,
        run_id,
        seq,
        "emergency_stop",
        "failed",
        Some("computer"),
        None,
        None,
        None,
        Some("Emergency stop engaged; pending approvals cancelled and execution aborted."),
        Some(&details),
    )
    .await;
}

/// Global emergency stop: halt every known run (panic button).
pub async fn emergency_stop_all(db: &Database) -> Vec<String> {
    let kernel = global_kernel(&db.pool);
    kernel.emergency_stop_all().await.unwrap_or_default()
}

/// List pending approvals (for the approval UI).
pub async fn list_pending_approvals(
    db: &Database,
    run_id: Option<i64>,
) -> Vec<overseer_kernel::ApprovalRecord> {
    let kernel = global_kernel(&db.pool);
    kernel
        .store()
        .list_pending(run_id)
        .await
        .unwrap_or_default()
}

#[cfg(test)]
mod bridge_tests {
    //! Integration test for the MAIN-CRATE bridge (not just the kernel crate):
    //! exercises `kernel_execute_with` against a real in-memory `Database`, proving
    //! the glue (policy build from supervision state, DbTraceSink → execution_events,
    //! suspend/approve/reject/emergency) with a recording/safe executor — no real
    //! desktop mutation.
    //!
    //! NOTE: the kernel is a process-global `OnceLock` bound to the first pool it
    //! sees, so all phases run within ONE test on ONE `Database`. This is the only
    //! main-crate test that touches the global kernel.
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    fn os_tool() -> ToolInfo {
        ToolInfo {
            id: 1,
            name: "os_tool".to_string(),
            description: "native system access".to_string(),
            code: String::new(),
            language: "rust".to_string(),
            script_path: None,
            required_intent: None,
            safety_level: "standard".to_string(),
            version: "1.0.0".to_string(),
        }
    }

    async fn event_types(db: &crate::db::Database, run_id: i64) -> Vec<String> {
        sqlx::query_scalar::<_, String>(
            "SELECT event_type FROM execution_events WHERE run_id = ? ORDER BY sequence",
        )
        .bind(run_id)
        .fetch_all(&db.pool)
        .await
        .unwrap_or_default()
    }

    async fn new_run(db: &crate::db::Database, q: &str) -> i64 {
        let id = db.next_id().await;
        sqlx::query("INSERT INTO execution_runs (id, user_query, status) VALUES (?, ?, 'running')")
            .bind(id)
            .bind(q)
            .execute(&db.pool)
            .await
            .unwrap();
        id
    }

    #[tokio::test]
    async fn bridge_governs_suspend_approve_reject_emergency() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        // approval_required is the default, but set it explicitly for clarity.
        let _ =
            crate::backend::set_setting(&db, "computer_supervision_mode", "approval_required").await;

        let args = serde_json::json!({"action":"click","x":10,"y":20});

        // ---------- Phase A: suspend, then approve → immutable execute once ----------
        let run_a = new_run(&db, "phase-a").await;
        let count = Arc::new(AtomicUsize::new(0));
        let captured = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
        {
            let (db2, c, cap, a) = (db.clone(), count.clone(), captured.clone(), args.clone());
            let handle = tokio::spawn(async move {
                kernel_execute_with(
                    &db2,
                    Some(run_a),
                    "computer",
                    "click",
                    &os_tool(),
                    a,
                    &["os_tool".to_string()],
                    move |req: ActionRequest| async move {
                        cap.lock().unwrap().push(req.arguments.clone());
                        c.fetch_add(1, Ordering::SeqCst);
                        Ok::<String, String>("clicked".to_string())
                    },
                )
                .await
            });

            // Must suspend with a pending approval and NOT execute.
            let mut approval_id = String::new();
            for _ in 0..200 {
                if let Some(r) = list_pending_approvals(&db, Some(run_a)).await.first() {
                    approval_id = r.id.clone();
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            assert!(!approval_id.is_empty(), "action should have suspended");
            assert_eq!(count.load(Ordering::SeqCst), 0, "no execution before approval");
            let evs = event_types(&db, run_a).await;
            assert!(evs.contains(&"action_requested".to_string()));
            assert!(evs.contains(&"approval_requested".to_string()));
            assert!(!evs.contains(&"action_started".to_string()));

            assert!(approve(&db, &approval_id, "tester"), "approve resolves it");
            let outcome = handle.await.unwrap();
            assert!(matches!(outcome, ExecutionOutcome::Executed { .. }));
            assert_eq!(count.load(Ordering::SeqCst), 1, "executes exactly once");
            assert_eq!(
                captured.lock().unwrap()[0],
                args,
                "the immutable staged payload executed"
            );
            let evs = event_types(&db, run_a).await;
            assert!(evs.contains(&"approval_approved".to_string()));
            assert!(evs.contains(&"action_started".to_string()));
            assert!(evs.contains(&"action_completed".to_string()));
        }

        // ---------- Phase B: reject → never executes ----------
        let run_b = new_run(&db, "phase-b").await;
        {
            let count_b = Arc::new(AtomicUsize::new(0));
            let (db2, c, a) = (db.clone(), count_b.clone(), args.clone());
            let handle = tokio::spawn(async move {
                kernel_execute_with(
                    &db2,
                    Some(run_b),
                    "computer",
                    "click",
                    &os_tool(),
                    a,
                    &["os_tool".to_string()],
                    move |_req: ActionRequest| async move {
                        c.fetch_add(1, Ordering::SeqCst);
                        Ok::<String, String>("clicked".to_string())
                    },
                )
                .await
            });
            let mut approval_id = String::new();
            for _ in 0..200 {
                if let Some(r) = list_pending_approvals(&db, Some(run_b)).await.first() {
                    approval_id = r.id.clone();
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            assert!(reject(&db, &approval_id, "tester"));
            let outcome = handle.await.unwrap();
            assert!(matches!(outcome, ExecutionOutcome::Rejected { .. }));
            assert_eq!(count_b.load(Ordering::SeqCst), 0, "rejected action never executes");
        }

        // ---------- Phase C: emergency stop cancels a pending action ----------
        let run_c = new_run(&db, "phase-c").await;
        {
            let count_c = Arc::new(AtomicUsize::new(0));
            let (db2, c, a) = (db.clone(), count_c.clone(), args.clone());
            let handle = tokio::spawn(async move {
                kernel_execute_with(
                    &db2,
                    Some(run_c),
                    "computer",
                    "click",
                    &os_tool(),
                    a,
                    &["os_tool".to_string()],
                    move |_req: ActionRequest| async move {
                        c.fetch_add(1, Ordering::SeqCst);
                        Ok::<String, String>("clicked".to_string())
                    },
                )
                .await
            });
            for _ in 0..200 {
                if !list_pending_approvals(&db, Some(run_c)).await.is_empty() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(5)).await;
            }
            emergency_stop(&db, run_c).await;
            let outcome = handle.await.unwrap();
            assert!(matches!(outcome, ExecutionOutcome::Cancelled { .. }));
            assert_eq!(count_c.load(Ordering::SeqCst), 0, "emergency-stopped action never executes");
            let evs = event_types(&db, run_c).await;
            assert!(evs.contains(&"emergency_stop".to_string()));
        }
    }
}
