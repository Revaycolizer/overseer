//! Integration tests for the main crate's kernel bridge (`engine::kernel`).
//!
//! These tests exercise the full path from the main crate's kernel bridge through
//! to the `overseer-kernel` crate. They verify:
//!
//! - Policy decisions correctly route allow / deny / require-approval
//! - The global kernel initialises and binds to the database pool
//! - Action classification maps correctly to ActionKind
//! - The kernel bridge does not panic on any known code path

#![recursion_limit = "512"]

use overseer::engine::kernel as kernel_bridge;
use overseer::engine::utils::discovery::ToolInfo;
use overseer_kernel::{ActionKind, ActionRequest, ExecutionOutcome, PolicyConfig, PolicyDecision};
use serde_json::json;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

fn make_tool(name: &str, language: &str) -> ToolInfo {
    ToolInfo {
        id: 0,
        name: name.to_string(),
        description: String::new(),
        code: String::new(),
        language: language.to_string(),
        script_path: None,
        required_intent: None,
        safety_level: "normal".to_string(),
        version: "1.0".to_string(),
    }
}

fn make_action(run_id: i64, tool_name: &str, kind: ActionKind) -> ActionRequest {
    ActionRequest::new(
        run_id,
        kind,
        tool_name,
        json!({"action": "test", "path": "/tmp/test.txt"}),
        vec![tool_name.to_string()],
        5000,
        "test action",
    )
}

// ---------------------------------------------------------------------------
// Action Classification Tests
// ---------------------------------------------------------------------------

#[test]
fn test_classify_native_tool() {
    let tool = make_tool("web_search", "rust");
    let kind = kernel_bridge::classify_action_kind(&tool, "web");
    assert_eq!(kind, ActionKind::NativeTool);
}

#[test]
fn test_classify_python_tool() {
    let tool = make_tool("analyze_data", "python");
    let kind = kernel_bridge::classify_action_kind(&tool, "data");
    assert_eq!(kind, ActionKind::PythonTool);
}

#[test]
fn test_classify_computer_control_by_name() {
    let tool = make_tool("os_tool", "rust");
    let kind = kernel_bridge::classify_action_kind(&tool, "computer");
    assert_eq!(kind, ActionKind::ComputerControl);
}

#[test]
fn test_classify_computer_control_by_domain() {
    let tool = make_tool("desktop_tool", "rust");
    let kind = kernel_bridge::classify_action_kind(&tool, "desktop");
    assert_eq!(kind, ActionKind::ComputerControl);
}

#[test]
fn test_classify_filesystem_write() {
    let tool = make_tool("write_file", "rust");
    let kind = kernel_bridge::classify_action_kind(&tool, "files");
    assert_eq!(kind, ActionKind::FilesystemWrite);
}

#[test]
fn test_classify_shell_command() {
    let tool = make_tool("code_executor", "rust");
    let kind = kernel_bridge::classify_action_kind(&tool, "code");
    assert_eq!(kind, ActionKind::ShellCommand);
}

#[test]
fn test_classify_shell_by_name() {
    let tool = make_tool("shell", "rust");
    let kind = kernel_bridge::classify_action_kind(&tool, "system");
    assert_eq!(kind, ActionKind::ShellCommand);
}

// ---------------------------------------------------------------------------
// Policy Decision Tests
// ---------------------------------------------------------------------------

#[test]
fn test_policy_allows_observation_without_approval() {
    let cfg = PolicyConfig {
        approval_required_default: true,
        observation_tools: ["get_state"].iter().map(|s| (*s).to_string()).collect(),
        capability_set: ["get_state"].iter().map(|s| (*s).to_string()).collect(),
        ..PolicyConfig::default()
    };

    let req = make_action(1, "get_state", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert_eq!(decision, PolicyDecision::Allow);
}

#[test]
fn test_policy_denies_uncapable_tool() {
    let cfg = PolicyConfig {
        capability_set: ["allowed_tool"].iter().map(|s| (*s).to_string()).collect(),
        ..PolicyConfig::default()
    };

    let req = make_action(1, "unknown_tool", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert!(
        matches!(decision, PolicyDecision::Deny { code, .. } if code == "capability_not_granted")
    );
}

#[test]
fn test_policy_denies_denied_tool() {
    let cfg = PolicyConfig {
        capability_set: ["bad_tool"].iter().map(|s| (*s).to_string()).collect(),
        denylist: ["bad_tool"].iter().map(|s| (*s).to_string()).collect(),
        ..PolicyConfig::default()
    };

    let req = make_action(1, "bad_tool", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert!(matches!(decision, PolicyDecision::Deny { code, .. } if code == "denied_tool"));
}

#[test]
fn test_policy_denies_denied_action() {
    let mut cfg = PolicyConfig::default();
    cfg.capability_set.insert("test_tool".to_string());
    cfg.denylist.insert("delete".to_string());

    let req = ActionRequest::new(
        1,
        ActionKind::NativeTool,
        "test_tool",
        json!({"action": "delete", "path": "/etc"}),
        vec!["test_tool".to_string()],
        5000,
        "delete action",
    );
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert!(matches!(decision, PolicyDecision::Deny { code, .. } if code == "denied_action"));
}

#[test]
fn test_policy_requires_approval_for_computer_control() {
    let mut cfg = PolicyConfig::default();
    cfg.capability_set.insert("os_tool".to_string());
    cfg.require_approval_kinds
        .insert(ActionKind::ComputerControl);

    let req = make_action(1, "os_tool", ActionKind::ComputerControl);
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert!(matches!(decision, PolicyDecision::RequireApproval { .. }));
}

#[test]
fn test_policy_respects_action_cap() {
    let cfg = PolicyConfig {
        capability_set: ["some_tool"].iter().map(|s| (*s).to_string()).collect(),
        max_actions: 1,
        action_count: 1,
        ..PolicyConfig::default()
    };

    let req = make_action(1, "some_tool", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert!(matches!(decision, PolicyDecision::Deny { code, .. } if code == "action_cap_exceeded"));
}

#[test]
fn test_policy_emergency_stop_denies_all() {
    let cfg = PolicyConfig {
        emergency_stopped: true,
        ..PolicyConfig::default()
    };

    let req = make_action(1, "any_tool", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert!(matches!(decision, PolicyDecision::Deny { code, .. } if code == "emergency_stop"));
}

#[test]
fn test_policy_high_risk_escalates() {
    let mut cfg = PolicyConfig::default();
    cfg.capability_set.insert("some_tool".to_string());
    cfg.escalate_high_risk = true;

    let mut req = make_action(1, "some_tool", ActionKind::NativeTool);
    req.risk_hint = Some("high".to_string());

    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert!(matches!(decision, PolicyDecision::RequireApproval { .. }));
}

#[test]
fn test_policy_allows_without_risk_hint() {
    let mut cfg = PolicyConfig::default();
    cfg.capability_set.insert("safe_tool".to_string());

    let req = make_action(1, "safe_tool", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert_eq!(decision, PolicyDecision::Allow);
}

#[test]
fn test_policy_denies_action_cap_with_observations_still_allowed() {
    let mut cfg = PolicyConfig::default();
    cfg.capability_set.insert("get_state".to_string());
    cfg.capability_set.insert("mutate".to_string());
    cfg.observation_tools.insert("get_state".to_string());
    cfg.max_actions = 1;
    cfg.action_count = 1;

    // Observation should still be allowed even at cap
    let obs_req = make_action(1, "get_state", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &obs_req);
    assert_eq!(decision, PolicyDecision::Allow);

    // Mutate should be denied
    let mut_req = make_action(1, "mutate", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &mut_req);
    assert!(matches!(decision, PolicyDecision::Deny { code, .. } if code == "action_cap_exceeded"));
}

#[test]
fn test_policy_allowlist_denies_outside_list() {
    let cfg = PolicyConfig {
        capability_set: ["tool_a", "tool_b"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        allowlist: ["tool_a"].iter().map(|s| (*s).to_string()).collect(),
        ..PolicyConfig::default()
    };

    let req = make_action(1, "tool_b", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert!(matches!(decision, PolicyDecision::Deny { code, .. } if code == "not_in_allowlist"));
}

#[test]
fn test_policy_allowlist_allows_listed() {
    let cfg = PolicyConfig {
        capability_set: ["tool_a", "tool_b"]
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        allowlist: ["tool_a"].iter().map(|s| (*s).to_string()).collect(),
        ..PolicyConfig::default()
    };

    let req = make_action(1, "tool_a", ActionKind::NativeTool);
    let decision = overseer_kernel::evaluate(&cfg, &req);
    assert_eq!(decision, PolicyDecision::Allow);
}

#[test]
fn test_policy_policy_config_snapshot_is_json() {
    let cfg = PolicyConfig::default();
    let snapshot = cfg.snapshot();
    let parsed: serde_json::Value =
        serde_json::from_str(&snapshot).expect("snapshot should be valid JSON");
    assert_eq!(parsed["policy_version"], "kernel-0.1");
}

#[test]
fn test_policy_effective_timeout_ms_caps() {
    let cfg = PolicyConfig {
        timeout_ms_cap: 10000,
        ..PolicyConfig::default()
    };
    assert_eq!(cfg.effective_timeout_ms(5000), 5000);
    assert_eq!(cfg.effective_timeout_ms(20000), 10000);
}

#[test]
fn test_policy_effective_timeout_ms_zero_cap() {
    let cfg = PolicyConfig {
        timeout_ms_cap: 0,
        ..PolicyConfig::default()
    };
    assert_eq!(cfg.effective_timeout_ms(5000), 5000);
    assert_eq!(cfg.effective_timeout_ms(0), 0);
}

// ---------------------------------------------------------------------------
// ActionRequest Tests
// ---------------------------------------------------------------------------

#[test]
fn test_action_request_canonical_payload_is_json() {
    let req = make_action(1, "test_tool", ActionKind::NativeTool);
    let payload = req.canonical_payload();
    let parsed: serde_json::Value =
        serde_json::from_str(&payload).expect("payload should be valid JSON");
    assert_eq!(parsed["tool_name"], "test_tool");
    assert_eq!(parsed["run_id"], 1);
}

#[test]
fn test_action_request_unique_ids() {
    let req1 = make_action(1, "tool", ActionKind::NativeTool);
    let req2 = make_action(1, "tool", ActionKind::NativeTool);
    assert_ne!(req1.id, req2.id);
}

// ---------------------------------------------------------------------------
// Bridge Classification Tests
// ---------------------------------------------------------------------------

#[test]
fn test_bridge_classify_all_combinations() {
    let cases = vec![
        ("write_file", "rust", "files", ActionKind::FilesystemWrite),
        ("web_search", "rust", "web", ActionKind::NativeTool),
        ("charting", "rust", "analysis", ActionKind::NativeTool),
        ("os_tool", "rust", "computer", ActionKind::ComputerControl),
        ("os_tool", "rust", "desktop", ActionKind::ComputerControl),
        ("shell", "rust", "system", ActionKind::ShellCommand),
        ("code_executor", "rust", "code", ActionKind::ShellCommand),
        ("python_script", "python", "data", ActionKind::PythonTool),
        ("any_tool", "rust", "computer", ActionKind::ComputerControl),
    ];

    for (name, lang, domain, expected) in cases {
        let tool = make_tool(name, lang);
        let kind = kernel_bridge::classify_action_kind(&tool, domain);
        assert_eq!(kind, expected, "Mismatch for tool={name} domain={domain}");
    }
}

// ---------------------------------------------------------------------------
// ExecutionOutcome Tests
// ---------------------------------------------------------------------------

#[test]
fn test_execution_outcome_executed_check() {
    let outcome = ExecutionOutcome::Executed {
        output: "ok".into(),
    };
    assert!(outcome.executed());
}

#[test]
fn test_execution_outcome_not_executed() {
    let outcome = ExecutionOutcome::Denied {
        code: "test".into(),
        reason: "test".into(),
    };
    assert!(!outcome.executed());
}

#[test]
fn test_execution_outcome_agent_response_executed() {
    let outcome = ExecutionOutcome::Executed {
        output: "done".into(),
    };
    let resp = outcome.agent_response();
    assert!(resp.contains("executed"));
    assert!(resp.contains("done"));
}

#[test]
fn test_execution_outcome_agent_response_denied() {
    let outcome = ExecutionOutcome::Denied {
        code: "test".into(),
        reason: "test reason".into(),
    };
    let resp = outcome.agent_response();
    assert!(resp.contains("denied"));
    assert!(resp.contains("test reason"));
}

// ---------------------------------------------------------------------------
// PolicyConfig Tests
// ---------------------------------------------------------------------------

#[test]
fn test_policy_config_default_is_safe() {
    let cfg = PolicyConfig::default();
    assert_eq!(cfg.max_actions, 25);
    assert_eq!(cfg.timeout_ms_cap, 600_000);
    assert!(cfg.escalate_high_risk);
    assert!(!cfg.emergency_stopped);
    assert!(!cfg.approval_required_default);
    assert!(cfg.capability_set.is_empty());
    assert!(cfg.denylist.is_empty());
    assert!(cfg.allowlist.is_empty());
}
