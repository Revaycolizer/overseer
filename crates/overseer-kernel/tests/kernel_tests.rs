//! Kernel behavior tests — the demonstrable proof that agent actions cannot
//! bypass deterministic policy enforcement.
//!
//! These run independently of the Dioxus UI (the whole point of the separate
//! kernel crate): they build an in-memory SQLite database and drive the kernel
//! directly with a recording executor whose invocation we can observe.

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;

use overseer_kernel::{
    evaluate, ActionKind, ActionRequest, ApprovalStore, ExecutionOutcome, Kernel, KernelEvent,
    KernelEventKind, PolicyConfig, PolicyDecision, Resolution, TraceSink,
};

// ---------- test harness ----------

async fn mem_pool() -> SqlitePool {
    // max_connections(1) keeps a single shared in-memory database.
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .expect("open in-memory sqlite");
    ApprovalStore::new(pool.clone())
        .ensure_schema()
        .await
        .expect("apply approval schema");
    pool
}

#[derive(Default)]
struct TestSink {
    events: Mutex<Vec<KernelEvent>>,
}

#[async_trait]
impl TraceSink for TestSink {
    async fn record(&self, event: KernelEvent) {
        self.events.lock().unwrap().push(event);
    }
}

impl TestSink {
    fn kinds(&self) -> Vec<KernelEventKind> {
        self.events.lock().unwrap().iter().map(|e| e.kind).collect()
    }
    fn has(&self, kind: KernelEventKind) -> bool {
        self.kinds().contains(&kind)
    }
}

/// A recording executor: counts invocations and captures the exact request it
/// ran, so tests can assert *whether* and *what* executed.
#[derive(Clone, Default)]
struct Recorder {
    count: Arc<AtomicUsize>,
    executed: Arc<Mutex<Vec<ActionRequest>>>,
}

impl Recorder {
    fn make(
        &self,
    ) -> impl FnOnce(
        ActionRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<String, String>> + Send>,
    > {
        let count = self.count.clone();
        let executed = self.executed.clone();
        move |req: ActionRequest| {
            Box::pin(async move {
                executed.lock().unwrap().push(req.clone());
                count.fetch_add(1, Ordering::SeqCst);
                Ok(format!("executed:{}", req.tool_name))
            })
        }
    }
    fn runs(&self) -> usize {
        self.count.load(Ordering::SeqCst)
    }
}

fn caps(names: &[&str]) -> HashSet<String> {
    names.iter().map(|s| s.to_string()).collect()
}

fn base_cfg(cap_names: &[&str]) -> PolicyConfig {
    PolicyConfig {
        capability_set: caps(cap_names),
        max_actions: 100,
        approval_required_default: false,
        ..PolicyConfig::default()
    }
}

fn computer_req(run_id: i64, tool: &str) -> ActionRequest {
    ActionRequest::new(
        run_id,
        ActionKind::ComputerControl,
        tool,
        serde_json::json!({"action": "click", "x": 100, "y": 200}),
        vec![tool.to_string()],
        5_000,
        format!("{} click", tool),
    )
}

async fn wait_for_pending(store: &ApprovalStore, run_id: i64) -> String {
    for _ in 0..200 {
        let pending = store.list_pending(Some(run_id)).await.unwrap();
        if let Some(rec) = pending.first() {
            return rec.id.clone();
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("no pending approval appeared for run {}", run_id);
}

// ---------- pure policy unit coverage (no I/O) ----------

#[test]
fn policy_denies_when_capability_missing() {
    let cfg = base_cfg(&[]); // os_tool not granted
    let req = computer_req(1, "os_tool");
    match evaluate(&cfg, &req) {
        PolicyDecision::Deny { code, .. } => assert_eq!(code, "capability_not_granted"),
        other => panic!("expected deny, got {:?}", other),
    }
}

#[test]
fn policy_emergency_stop_denies_everything() {
    let mut cfg = base_cfg(&["os_tool"]);
    cfg.emergency_stopped = true;
    match evaluate(&cfg, &computer_req(1, "os_tool")) {
        PolicyDecision::Deny { code, .. } => assert_eq!(code, "emergency_stop"),
        other => panic!("expected deny, got {:?}", other),
    }
}

#[test]
fn policy_action_cap_denies_next() {
    let mut cfg = base_cfg(&["os_tool"]);
    cfg.max_actions = 1;
    cfg.action_count = 1;
    match evaluate(&cfg, &computer_req(1, "os_tool")) {
        PolicyDecision::Deny { code, .. } => assert_eq!(code, "action_cap_exceeded"),
        other => panic!("expected deny, got {:?}", other),
    }
}

#[test]
fn policy_denylist_beats_allow() {
    let mut cfg = base_cfg(&["os_tool"]);
    cfg.denylist = caps(&["os_tool"]);
    assert!(matches!(
        evaluate(&cfg, &computer_req(1, "os_tool")),
        PolicyDecision::Deny { .. }
    ));
}

#[test]
fn policy_high_risk_hint_escalates_but_never_authorizes() {
    // risk_hint can only escalate allow -> require_approval, never grant.
    let cfg = base_cfg(&["os_tool"]);
    let mut req = computer_req(1, "os_tool");
    req.risk_hint = Some("high".into());
    assert!(matches!(
        evaluate(&cfg, &req),
        PolicyDecision::RequireApproval { .. }
    ));
    // Even a "low" hint cannot bypass a missing capability.
    let cfg_nocap = base_cfg(&[]);
    let mut req2 = computer_req(1, "os_tool");
    req2.risk_hint = Some("low".into());
    assert!(matches!(
        evaluate(&cfg_nocap, &req2),
        PolicyDecision::Deny { .. }
    ));
}

// ---------- required scenario 1 ----------

#[tokio::test]
async fn scenario_01_allowed_action_executes_once() {
    let kernel = Kernel::new(mem_pool().await, None);
    let sink = TestSink::default();
    let rec = Recorder::default();

    let outcome = kernel
        .authorize_and_execute(
            base_cfg(&["os_tool"]),
            computer_req(1, "os_tool"),
            &sink,
            rec.make(),
        )
        .await;

    assert!(matches!(outcome, ExecutionOutcome::Executed { .. }));
    assert_eq!(rec.runs(), 1, "executor must run exactly once");
    assert_eq!(
        kernel.current_action_count(1),
        1,
        "action counter incremented"
    );
    assert!(sink.has(KernelEventKind::PolicyAllowed));
    assert!(sink.has(KernelEventKind::ActionCompleted));
}

// ---------- required scenario 2 ----------

#[tokio::test]
async fn scenario_02_denied_action_never_reaches_executor() {
    let kernel = Kernel::new(mem_pool().await, None);
    let sink = TestSink::default();
    let rec = Recorder::default();

    // Capability not granted -> deny.
    let outcome = kernel
        .authorize_and_execute(base_cfg(&[]), computer_req(1, "os_tool"), &sink, rec.make())
        .await;

    assert!(matches!(outcome, ExecutionOutcome::Denied { .. }));
    assert_eq!(rec.runs(), 0, "executor must NOT run on deny");
    assert_eq!(kernel.current_action_count(1), 0);
    assert!(sink.has(KernelEventKind::PolicyDenied));
    assert!(!sink.has(KernelEventKind::ActionStarted));
}

// ---------- required scenarios 3 & 4 ----------

#[tokio::test]
async fn scenario_03_04_require_approval_waits_then_executes_immutable_payload() {
    let kernel = Arc::new(Kernel::new(mem_pool().await, None));
    let sink = Arc::new(TestSink::default());
    let rec = Recorder::default();

    let mut cfg = base_cfg(&["os_tool"]);
    cfg.approval_required_default = true;

    let original = computer_req(42, "os_tool");
    let original_args = original.arguments.clone();
    let original_id = original.id.clone();

    let k = kernel.clone();
    let s = sink.clone();
    let exec = rec.make();
    let handle = tokio::spawn(async move {
        k.authorize_and_execute(cfg, original, s.as_ref(), exec)
            .await
    });

    // 3. It must NOT execute before approval.
    let approval_id = wait_for_pending(kernel.store(), 42).await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    assert_eq!(rec.runs(), 0, "must not execute before approval");
    assert!(sink.has(KernelEventKind::ApprovalRequested));

    // Approve.
    assert!(kernel.resolve_approval(
        &approval_id,
        Resolution::Approved {
            resolver: Some("tester".into())
        }
    ));

    let outcome = handle.await.unwrap();
    assert!(matches!(outcome, ExecutionOutcome::Executed { .. }));
    assert_eq!(rec.runs(), 1);

    // 4. The executed payload is the immutable staged one.
    let (exec_id, exec_args) = {
        let executed = rec.executed.lock().unwrap();
        assert_eq!(executed.len(), 1);
        (executed[0].id.clone(), executed[0].arguments.clone())
    };
    assert_eq!(exec_id, original_id, "same action id executed");
    assert_eq!(exec_args, original_args, "same arguments executed");

    // Ledger reflects approval.
    let rec_row = kernel.store().get(&approval_id).await.unwrap().unwrap();
    assert_eq!(rec_row.status, "approved");
    assert_eq!(rec_row.resolver.as_deref(), Some("tester"));
}

// ---------- required scenario 5 ----------

#[tokio::test]
async fn scenario_05_rejected_action_never_executes() {
    let kernel = Arc::new(Kernel::new(mem_pool().await, None));
    let sink = Arc::new(TestSink::default());
    let rec = Recorder::default();

    let mut cfg = base_cfg(&["os_tool"]);
    cfg.approval_required_default = true;

    let k = kernel.clone();
    let s = sink.clone();
    let exec = rec.make();
    let handle = tokio::spawn(async move {
        k.authorize_and_execute(cfg, computer_req(7, "os_tool"), s.as_ref(), exec)
            .await
    });

    let approval_id = wait_for_pending(kernel.store(), 7).await;
    assert!(kernel.resolve_approval(
        &approval_id,
        Resolution::Rejected {
            resolver: Some("tester".into())
        }
    ));

    let outcome = handle.await.unwrap();
    assert!(matches!(outcome, ExecutionOutcome::Rejected { .. }));
    assert_eq!(rec.runs(), 0, "rejected action must never execute");
    let rec_row = kernel.store().get(&approval_id).await.unwrap().unwrap();
    assert_eq!(rec_row.status, "rejected");
}

// ---------- required scenario 6 ----------

#[tokio::test]
async fn scenario_06_duplicate_resolution_is_idempotent() {
    let kernel = Arc::new(Kernel::new(mem_pool().await, None));
    let sink = Arc::new(TestSink::default());
    let rec = Recorder::default();

    let mut cfg = base_cfg(&["os_tool"]);
    cfg.approval_required_default = true;

    let k = kernel.clone();
    let s = sink.clone();
    let exec = rec.make();
    let handle = tokio::spawn(async move {
        k.authorize_and_execute(cfg, computer_req(9, "os_tool"), s.as_ref(), exec)
            .await
    });

    let approval_id = wait_for_pending(kernel.store(), 9).await;

    let first = kernel.resolve_approval(&approval_id, Resolution::Approved { resolver: None });
    let second = kernel.resolve_approval(&approval_id, Resolution::Rejected { resolver: None });
    assert!(first, "first resolution wins");
    assert!(!second, "duplicate resolution is rejected");

    let outcome = handle.await.unwrap();
    assert!(matches!(outcome, ExecutionOutcome::Executed { .. }));
    assert_eq!(
        rec.runs(),
        1,
        "executes exactly once despite duplicate resolve"
    );
}

// ---------- required scenario 7 ----------

#[tokio::test]
async fn scenario_07_action_cap_blocks_next_action() {
    let kernel = Kernel::new(mem_pool().await, None);
    let sink = TestSink::default();
    let rec = Recorder::default();

    let mut cfg = base_cfg(&["os_tool"]);
    cfg.max_actions = 1;

    // First action allowed and executed (count -> 1).
    let o1 = kernel
        .authorize_and_execute(cfg.clone(), computer_req(3, "os_tool"), &sink, rec.make())
        .await;
    assert!(matches!(o1, ExecutionOutcome::Executed { .. }));

    // Second action denied by the enforced cap (kernel injects action_count=1).
    let o2 = kernel
        .authorize_and_execute(cfg.clone(), computer_req(3, "os_tool"), &sink, rec.make())
        .await;
    match o2 {
        ExecutionOutcome::Denied { code, .. } => assert_eq!(code, "action_cap_exceeded"),
        other => panic!("expected cap denial, got {:?}", other),
    }
    assert_eq!(rec.runs(), 1, "only the first action executed");
}

// ---------- required scenario 8 ----------

#[tokio::test]
async fn scenario_08_timeout_terminates_and_records_event() {
    let kernel = Kernel::new(mem_pool().await, None);
    let sink = TestSink::default();
    let count = Arc::new(AtomicUsize::new(0));

    let mut req = computer_req(11, "os_tool");
    req.timeout_ms = 40; // shorter than the executor's sleep

    let c = count.clone();
    let slow_exec = move |_req: ActionRequest| {
        let c = c.clone();
        async move {
            tokio::time::sleep(Duration::from_millis(500)).await;
            c.fetch_add(1, Ordering::SeqCst);
            Ok::<String, String>("late".into())
        }
    };

    let outcome = kernel
        .authorize_and_execute(base_cfg(&["os_tool"]), req, &sink, slow_exec)
        .await;

    assert!(matches!(outcome, ExecutionOutcome::TimedOut));
    assert_eq!(
        count.load(Ordering::SeqCst),
        0,
        "slow body was aborted before completing"
    );
    assert!(sink.has(KernelEventKind::ActionTimedOut));
    assert_eq!(
        kernel.current_action_count(11),
        0,
        "timed-out action does not count"
    );
}

// ---------- required scenario 9 ----------

#[tokio::test]
async fn scenario_09_emergency_stop_cancels_pending_approval() {
    let kernel = Arc::new(Kernel::new(mem_pool().await, None));
    let sink = Arc::new(TestSink::default());
    let rec = Recorder::default();

    let mut cfg = base_cfg(&["os_tool"]);
    cfg.approval_required_default = true;

    let k = kernel.clone();
    let s = sink.clone();
    let exec = rec.make();
    let handle = tokio::spawn(async move {
        k.authorize_and_execute(cfg, computer_req(5, "os_tool"), s.as_ref(), exec)
            .await
    });

    let approval_id = wait_for_pending(kernel.store(), 5).await;

    let cancelled = kernel.emergency_stop(5).await.unwrap();
    assert!(
        cancelled.contains(&approval_id),
        "emergency stop cancels the pending approval"
    );

    let outcome = handle.await.unwrap();
    assert!(matches!(outcome, ExecutionOutcome::Cancelled { .. }));
    assert_eq!(rec.runs(), 0, "cancelled action never executes");

    let rec_row = kernel.store().get(&approval_id).await.unwrap().unwrap();
    assert_eq!(rec_row.status, "cancelled");

    // And a subsequent action on the emergency-stopped run is denied.
    let after = kernel
        .authorize_and_execute(
            base_cfg(&["os_tool"]),
            computer_req(5, "os_tool"),
            sink.as_ref(),
            rec.make(),
        )
        .await;
    match after {
        ExecutionOutcome::Denied { code, .. } => assert_eq!(code, "emergency_stop"),
        other => panic!("expected emergency deny, got {:?}", other),
    }
}

// ---------- required scenario 10 ----------

#[tokio::test]
async fn scenario_10_no_execution_without_policy_pass() {
    // The kernel API only ever invokes the executor from inside
    // authorize_and_execute, and only after policy allow/approval. We prove the
    // negative: across deny, reject, cancel and emergency, the executor is never
    // invoked — there is no path to execution that skips policy.
    let kernel = Arc::new(Kernel::new(mem_pool().await, None));
    let sink = Arc::new(TestSink::default());
    let rec = Recorder::default();

    // deny (no capability)
    let _ = kernel
        .authorize_and_execute(
            base_cfg(&[]),
            computer_req(1, "os_tool"),
            sink.as_ref(),
            rec.make(),
        )
        .await;

    // reject
    {
        let mut cfg = base_cfg(&["os_tool"]);
        cfg.approval_required_default = true;
        let k = kernel.clone();
        let s = sink.clone();
        let exec = rec.make();
        let handle = tokio::spawn(async move {
            k.authorize_and_execute(cfg, computer_req(2, "os_tool"), s.as_ref(), exec)
                .await
        });
        let id = wait_for_pending(kernel.store(), 2).await;
        kernel.resolve_approval(&id, Resolution::Rejected { resolver: None });
        let _ = handle.await.unwrap();
    }

    // emergency-stopped run
    kernel.emergency_stop(3).await.unwrap();
    let _ = kernel
        .authorize_and_execute(
            base_cfg(&["os_tool"]),
            computer_req(3, "os_tool"),
            sink.as_ref(),
            rec.make(),
        )
        .await;

    assert_eq!(
        rec.runs(),
        0,
        "executor never ran on any non-authorized path"
    );
}

// ---------- extra coverage: restart recovery & expiry ----------

#[tokio::test]
async fn restart_recovery_cancels_stale_pending() {
    let pool = mem_pool().await;
    let store = ApprovalStore::new(pool.clone());
    let req = computer_req(1, "os_tool");
    store
        .create_pending(&req, "needs approval", None, "kernel-0.1", "{}")
        .await
        .unwrap();
    assert_eq!(store.list_pending(None).await.unwrap().len(), 1);

    // Simulate a fresh process taking over the same database.
    let kernel = Kernel::new(pool.clone(), None);
    let recovered = kernel.recover_pending_on_startup().await.unwrap();
    assert_eq!(recovered, 1);
    assert!(store.list_pending(None).await.unwrap().is_empty());
}

#[tokio::test]
async fn recovery_is_idempotent_and_preserves_resolved() {
    let pool = mem_pool().await;
    let store = ApprovalStore::new(pool.clone());
    // one pending, one already-approved row
    let pending = computer_req(1, "os_tool");
    store
        .create_pending(&pending, "needs approval", None, "kernel-0.1", "{}")
        .await
        .unwrap();
    let resolved = computer_req(1, "os_tool");
    let resolved_id = store
        .create_pending(&resolved, "needs approval", None, "kernel-0.1", "{}")
        .await
        .unwrap();
    assert!(store
        .mark_resolved(
            &resolved_id,
            overseer_kernel::ApprovalStatus::Approved,
            Some("human"),
            Some("approved")
        )
        .await
        .unwrap());

    let kernel = Kernel::new(pool.clone(), None);
    let first = kernel.recover_pending_on_startup().await.unwrap();
    assert_eq!(first, 1, "only the pending row is cancelled");
    // Second boot: nothing left to cancel — does not rewrite resolved rows.
    let second = kernel.recover_pending_on_startup().await.unwrap();
    assert_eq!(second, 0, "recovery is idempotent");

    let resolved_row = store.get(&resolved_id).await.unwrap().unwrap();
    assert_eq!(resolved_row.status, "approved", "resolved row untouched");
    assert_eq!(resolved_row.resolver.as_deref(), Some("human"));
}

#[tokio::test]
async fn distinct_approvals_cannot_cross_resolve() {
    let kernel = Arc::new(Kernel::new(mem_pool().await, None));
    let sink = Arc::new(TestSink::default());
    let rec_a = Recorder::default();
    let rec_b = Recorder::default();

    let mut cfg = base_cfg(&["os_tool"]);
    cfg.approval_required_default = true;

    // Two independent pending approvals on two runs.
    let (ka, sa, ea) = (kernel.clone(), sink.clone(), rec_a.make());
    let ha = tokio::spawn(async move {
        ka.authorize_and_execute(cfg.clone(), computer_req(100, "os_tool"), sa.as_ref(), ea)
            .await
    });
    let mut cfg_b = base_cfg(&["os_tool"]);
    cfg_b.approval_required_default = true;
    let (kb, sb, eb) = (kernel.clone(), sink.clone(), rec_b.make());
    let hb = tokio::spawn(async move {
        kb.authorize_and_execute(cfg_b, computer_req(200, "os_tool"), sb.as_ref(), eb)
            .await
    });

    let id_a = wait_for_pending(kernel.store(), 100).await;
    let _id_b = wait_for_pending(kernel.store(), 200).await;

    // Approving A must not resolve or execute B.
    assert!(kernel.resolve_approval(&id_a, Resolution::Approved { resolver: None }));
    let outcome_a = ha.await.unwrap();
    assert!(matches!(outcome_a, ExecutionOutcome::Executed { .. }));
    assert_eq!(rec_a.runs(), 1);
    assert_eq!(rec_b.runs(), 0, "B must not execute when A is approved");

    // B is still pending and independently rejectable.
    let pending_b = kernel.store().list_pending(Some(200)).await.unwrap();
    assert_eq!(pending_b.len(), 1);
    kernel.resolve_approval(&pending_b[0].id, Resolution::Rejected { resolver: None });
    let outcome_b = hb.await.unwrap();
    assert!(matches!(outcome_b, ExecutionOutcome::Rejected { .. }));
    assert_eq!(rec_b.runs(), 0);
}

#[tokio::test]
async fn emergency_stop_wakes_all_waiters_in_run() {
    let kernel = Arc::new(Kernel::new(mem_pool().await, None));
    let sink = Arc::new(TestSink::default());
    let rec = Recorder::default();

    let mut cfg = base_cfg(&["os_tool"]);
    cfg.approval_required_default = true;

    // Two pending approvals on the SAME run.
    let (k1, s1, e1) = (kernel.clone(), sink.clone(), rec.make());
    let h1 = tokio::spawn(async move {
        k1.authorize_and_execute(cfg.clone(), computer_req(300, "os_tool"), s1.as_ref(), e1)
            .await
    });
    // Wait for the first to register before spawning the second (deterministic).
    let _ = wait_for_pending(kernel.store(), 300).await;
    let mut cfg2 = base_cfg(&["os_tool"]);
    cfg2.approval_required_default = true;
    let (k2, s2, e2) = (kernel.clone(), sink.clone(), rec.make());
    let h2 = tokio::spawn(async move {
        k2.authorize_and_execute(cfg2, computer_req(300, "os_tool"), s2.as_ref(), e2)
            .await
    });
    // Ensure both are pending.
    for _ in 0..200 {
        if kernel.store().list_pending(Some(300)).await.unwrap().len() >= 2 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let cancelled = kernel.emergency_stop(300).await.unwrap();
    assert_eq!(
        cancelled.len(),
        2,
        "emergency stop wakes all waiters in the run"
    );

    let o1 = h1.await.unwrap();
    let o2 = h2.await.unwrap();
    assert!(matches!(o1, ExecutionOutcome::Cancelled { .. }));
    assert!(matches!(o2, ExecutionOutcome::Cancelled { .. }));
    assert_eq!(rec.runs(), 0);
}

#[tokio::test]
async fn approval_expires_when_no_human_responds() {
    let kernel = Kernel::new(mem_pool().await, Some(Duration::from_millis(50)));
    let sink = TestSink::default();
    let rec = Recorder::default();

    let mut cfg = base_cfg(&["os_tool"]);
    cfg.approval_required_default = true;

    let outcome = kernel
        .authorize_and_execute(cfg, computer_req(1, "os_tool"), &sink, rec.make())
        .await;

    assert!(matches!(outcome, ExecutionOutcome::Expired));
    assert_eq!(rec.runs(), 0);
    let pending = kernel.store().list_pending(None).await.unwrap();
    assert!(pending.is_empty(), "expired approval is no longer pending");
}
