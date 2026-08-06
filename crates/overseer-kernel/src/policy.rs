//! Deterministic policy engine.
//!
//! [`evaluate`] is a **pure function**: given the same [`PolicyConfig`] and
//! [`ActionRequest`] it always returns the same [`PolicyDecision`]. It never
//! calls an LLM and never performs I/O. LLM heuristics may populate
//! [`ActionRequest::risk_hint`], but only deterministic rules here decide
//! authority.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::action::{ActionKind, ActionRequest};

/// Version tag recorded with every persisted approval so a decision can be
/// traced back to the policy rules that produced it.
pub const KERNEL_POLICY_VERSION: &str = "kernel-0.1";

/// The three — and only three — decisions the policy engine can return.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum PolicyDecision {
    /// Execute immediately.
    Allow,
    /// Never execute; return a structured denial to the agent.
    Deny { code: String, reason: String },
    /// Persist an approval request and pause until a human resolves it.
    RequireApproval {
        reason: String,
        required_role: Option<String>,
    },
}

/// Deterministic risk tier of an action, computed from its kind + arguments.
///
/// Ordering matters: `Low < Medium < High`. Policy may *auto-approve* actions at
/// or below a configured threshold (for unattended/autonomous runs), but High is
/// never auto-approved, and this classification is authoritative — it is derived
/// only from the request, never from the LLM's `risk_hint`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionRisk {
    Low,
    Medium,
    High,
}

/// Shell command heads considered read-only/safe.
const SAFE_SHELL_HEADS: &[&str] = &[
    "ls", "cat", "pwd", "echo", "find", "grep", "rg", "head", "tail", "wc", "stat",
    "file", "which", "whoami", "date", "uname", "df", "du", "ps", "env", "printenv",
    "hostname", "id", "uptime", "tree", "realpath", "dirname", "basename", "sort",
    "uniq", "cut", "cd",
];

/// Tokens that make a shell command dangerous regardless of the leading command
/// (mutation, escalation, network, redirection, chaining, substitution).
const DANGEROUS_SHELL_TOKENS: &[&str] = &[
    "rm ", "rm-", "rmdir", "mv ", "dd ", "mkfs", "chmod", "chown", "kill", "pkill",
    "shutdown", "reboot", "sudo", "curl", "wget", "ssh ", "scp ", "passwd", "truncate",
    "tee ", "mount", "umount", "systemctl", "service ", "crontab", "eval ", "apt",
    "yum", "dnf", "pacman", "pip ", "npm ", "git push", ">", "|", ";", "&", "`", "$(",
];

/// Native tools that are read-only and therefore low risk.
const LOW_NATIVE_TOOLS: &[&str] = &[
    "web_extract", "web_search", "web_links", "browser_read", "clipboard_read",
    "vision_analysis",
];

fn classify_shell_risk(command: &str) -> ActionRisk {
    let lower = command.to_lowercase();
    if DANGEROUS_SHELL_TOKENS.iter().any(|t| lower.contains(t)) {
        return ActionRisk::High;
    }
    let head = lower.split_whitespace().next().unwrap_or("");
    let head = head.rsplit('/').next().unwrap_or(head);
    if SAFE_SHELL_HEADS.contains(&head) {
        ActionRisk::Low
    } else {
        // Unknown command with no obvious danger token — still treat as risky.
        ActionRisk::High
    }
}

/// Authoritative, deterministic risk classification for an action.
#[must_use]
pub fn classify_risk(req: &ActionRequest) -> ActionRisk {
    match req.kind {
        ActionKind::PythonTool => ActionRisk::High,
        ActionKind::ShellCommand => {
            let cmd = req
                .arguments
                .get("command")
                .or_else(|| req.arguments.get("input").and_then(|i| i.get("command")))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            classify_shell_risk(cmd)
        }
        ActionKind::FilesystemWrite => ActionRisk::Medium,
        ActionKind::ComputerControl => ActionRisk::Medium,
        ActionKind::NativeTool => {
            if LOW_NATIVE_TOOLS.contains(&req.tool_name.as_str()) {
                ActionRisk::Low
            } else {
                ActionRisk::Medium
            }
        }
    }
}

/// The deterministic inputs to a policy decision for one run/session.
#[derive(Debug, Clone)]
pub struct PolicyConfig {
    /// Tools this agent/supervisor is permitted to use at all.
    pub capability_set: HashSet<String>,
    /// If non-empty, acts as a strict allow-list: only listed tool names may run.
    pub allowlist: HashSet<String>,
    /// Tools/actions that are always denied.
    pub denylist: HashSet<String>,
    /// Maximum number of successfully executed actions per run/session.
    pub max_actions: u64,
    /// How many actions have already executed in this run/session.
    pub action_count: u64,
    /// Upper bound on an action's execution timeout (ms). 0 = no cap.
    pub timeout_ms_cap: u64,
    /// True when computer supervision is in emergency-stop.
    pub emergency_stopped: bool,
    /// Action kinds that always require human approval.
    pub require_approval_kinds: HashSet<ActionKind>,
    /// When true (supervision_mode = "approval_required"), every side-effecting
    /// action requires approval unless it is a pure observation.
    pub approval_required_default: bool,
    /// Tool names treated as read-only observation (never require approval,
    /// never counted as risky actions), e.g. `vision_analysis`, `get_state`.
    pub observation_tools: HashSet<String>,
    /// Escalate to approval when `risk_hint == "high"`. Never downgrades.
    pub escalate_high_risk: bool,
    /// For unattended/autonomous runs: auto-approve actions whose deterministic
    /// [`classify_risk`] is at or below this tier, instead of pausing for a human.
    /// `None` (the default) never auto-approves — every gated action waits for a
    /// human. High-risk actions are never auto-approved regardless of this value.
    pub auto_approve_max_risk: Option<ActionRisk>,
}

impl Default for PolicyConfig {
    fn default() -> Self {
        Self {
            capability_set: HashSet::new(),
            allowlist: HashSet::new(),
            denylist: HashSet::new(),
            max_actions: 25,
            action_count: 0,
            timeout_ms_cap: 600_000,
            emergency_stopped: false,
            require_approval_kinds: HashSet::new(),
            approval_required_default: false,
            observation_tools: HashSet::new(),
            escalate_high_risk: true,
            auto_approve_max_risk: None,
        }
    }
}

impl PolicyConfig {
    fn is_observation(&self, req: &ActionRequest) -> bool {
        self.observation_tools.contains(&req.tool_name)
    }

    /// A compact, non-sensitive snapshot of the policy for the audit ledger.
    #[must_use]
    pub fn snapshot(&self) -> String {
        serde_json::json!({
            "policy_version": KERNEL_POLICY_VERSION,
            "capability_count": self.capability_set.len(),
            "allowlist_count": self.allowlist.len(),
            "denylist_count": self.denylist.len(),
            "max_actions": self.max_actions,
            "action_count": self.action_count,
            "timeout_ms_cap": self.timeout_ms_cap,
            "approval_required_default": self.approval_required_default,
        })
        .to_string()
    }

    /// The effective, capped execution timeout for an action.
    #[must_use]
    pub fn effective_timeout_ms(&self, requested: u64) -> u64 {
        let requested = if requested == 0 {
            self.timeout_ms_cap
        } else {
            requested
        };
        if self.timeout_ms_cap == 0 {
            requested
        } else {
            requested.min(self.timeout_ms_cap)
        }
    }
}

/// Deterministically evaluate one action against the policy.
///
/// Ordering is intentional and defense-in-depth: hard stops (emergency,
/// capability, denylist, allowlist, action cap) are checked before any
/// approval path, so a `deny` can never be softened into `require_approval`.
#[must_use]
pub fn evaluate(cfg: &PolicyConfig, req: &ActionRequest) -> PolicyDecision {
    // 1. Emergency stop halts everything.
    if cfg.emergency_stopped {
        return PolicyDecision::Deny {
            code: "emergency_stop".into(),
            reason: "Kernel is in emergency stop; no actions may run.".into(),
        };
    }

    // 2. Capability membership: the tool must be granted to this agent at all.
    if !cfg.capability_set.contains(&req.tool_name) {
        return PolicyDecision::Deny {
            code: "capability_not_granted".into(),
            reason: format!(
                "Tool '{}' is not in the agent's capability set.",
                req.tool_name
            ),
        };
    }

    // 3. Explicit denylist (tool name or the action verb inside arguments).
    if cfg.denylist.contains(&req.tool_name) {
        return PolicyDecision::Deny {
            code: "denied_tool".into(),
            reason: format!("Tool '{}' is explicitly denied.", req.tool_name),
        };
    }
    if let Some(action) = req.arguments.get("action").and_then(|v| v.as_str()) {
        if cfg.denylist.contains(action) {
            return PolicyDecision::Deny {
                code: "denied_action".into(),
                reason: format!("Action '{}' is explicitly denied.", action),
            };
        }
    }

    // 4. Strict allow-list (when configured, only listed tools may run).
    if !cfg.allowlist.is_empty() && !cfg.allowlist.contains(&req.tool_name) {
        return PolicyDecision::Deny {
            code: "not_in_allowlist".into(),
            reason: format!("Tool '{}' is not in the active allowlist.", req.tool_name),
        };
    }

    let is_observation = cfg.is_observation(req);

    // 5. Action cap (observations do not consume the budget).
    if !is_observation && cfg.action_count >= cfg.max_actions {
        return PolicyDecision::Deny {
            code: "action_cap_exceeded".into(),
            reason: format!(
                "Action cap reached ({}/{}).",
                cfg.action_count, cfg.max_actions
            ),
        };
    }

    // 6. Approval requirements (deterministic; risk_hint may only escalate).
    if !is_observation {
        let kind_needs_approval = cfg.require_approval_kinds.contains(&req.kind);
        let mode_needs_approval = cfg.approval_required_default;
        let high_risk_escalation = cfg.escalate_high_risk
            && req
                .risk_hint
                .as_deref()
                .map(|r| r.eq_ignore_ascii_case("high"))
                .unwrap_or(false);

        if kind_needs_approval || mode_needs_approval || high_risk_escalation {
            // Autonomous auto-approve: if a risk threshold is configured, an action
            // whose deterministic risk is at/below it runs without a human. This
            // NEVER applies to an LLM-flagged high-risk action, and only reaches
            // here after all hard-stop denies (emergency/capability/denylist/
            // allowlist/cap) have already passed.
            if !high_risk_escalation {
                if let Some(max) = cfg.auto_approve_max_risk {
                    if classify_risk(req) <= max {
                        return PolicyDecision::Allow;
                    }
                }
            }
            let reason = if high_risk_escalation && !(kind_needs_approval || mode_needs_approval) {
                format!(
                    "High-risk action '{}' requires human approval.",
                    req.tool_name
                )
            } else {
                format!(
                    "Action '{}' ({}) requires human approval.",
                    req.tool_name,
                    req.kind.as_str()
                )
            };
            return PolicyDecision::RequireApproval {
                reason,
                required_role: None,
            };
        }
    }

    // 7. Otherwise allowed.
    PolicyDecision::Allow
}

#[cfg(test)]
mod risk_tests {
    use super::*;

    fn req(kind: ActionKind, tool: &str, args: serde_json::Value) -> ActionRequest {
        ActionRequest::new(1, kind, tool, args, vec![tool.to_string()], 1000, "")
    }

    fn autonomous_cfg(tool: &str, max: ActionRisk) -> PolicyConfig {
        PolicyConfig {
            capability_set: [tool.to_string()].into_iter().collect(),
            require_approval_kinds: [
                ActionKind::ShellCommand,
                ActionKind::ComputerControl,
                ActionKind::FilesystemWrite,
            ]
            .into_iter()
            .collect(),
            approval_required_default: true,
            auto_approve_max_risk: Some(max),
            ..Default::default()
        }
    }

    #[test]
    fn shell_reads_are_low_writes_are_high() {
        assert_eq!(classify_risk(&req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"ls -la /tmp"}))), ActionRisk::Low);
        assert_eq!(classify_risk(&req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"cat notes.txt"}))), ActionRisk::Low);
        assert_eq!(classify_risk(&req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"rm -rf /tmp/x"}))), ActionRisk::High);
        assert_eq!(classify_risk(&req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"echo hi > f"}))), ActionRisk::High, "redirection is dangerous");
        assert_eq!(classify_risk(&req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"sudo apt install x"}))), ActionRisk::High);
        assert_eq!(classify_risk(&req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"frobnicate --all"}))), ActionRisk::High, "unknown command is risky");
    }

    #[test]
    fn kinds_have_expected_tiers() {
        assert_eq!(classify_risk(&req(ActionKind::FilesystemWrite, "write_file", serde_json::json!({}))), ActionRisk::Medium);
        assert_eq!(classify_risk(&req(ActionKind::ComputerControl, "os_tool", serde_json::json!({"action":"click"}))), ActionRisk::Medium);
        assert_eq!(classify_risk(&req(ActionKind::PythonTool, "py", serde_json::json!({}))), ActionRisk::High);
        assert_eq!(classify_risk(&req(ActionKind::NativeTool, "web_extract", serde_json::json!({}))), ActionRisk::Low);
        assert_eq!(classify_risk(&req(ActionKind::NativeTool, "browser_click", serde_json::json!({}))), ActionRisk::Medium);
    }

    #[test]
    fn autonomous_low_auto_approves_safe_shell_but_not_writes() {
        // Low threshold: `ls` runs; a file write still needs approval.
        let cfg = autonomous_cfg("shell_exec", ActionRisk::Low);
        assert_eq!(
            evaluate(&cfg, &req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"ls"}))),
            PolicyDecision::Allow
        );
        let wcfg = autonomous_cfg("write_file", ActionRisk::Low);
        assert!(matches!(
            evaluate(&wcfg, &req(ActionKind::FilesystemWrite, "write_file", serde_json::json!({"path":"/x","content":"y"}))),
            PolicyDecision::RequireApproval { .. }
        ));
        // A destructive shell command is never auto-approved at Low.
        assert!(matches!(
            evaluate(&cfg, &req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"rm -rf /"}))),
            PolicyDecision::RequireApproval { .. }
        ));
    }

    #[test]
    fn high_risk_hint_is_never_auto_approved() {
        let cfg = autonomous_cfg("shell_exec", ActionRisk::High); // even max threshold
        let mut r = req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"ls"}));
        r.risk_hint = Some("high".into());
        assert!(matches!(evaluate(&cfg, &r), PolicyDecision::RequireApproval { .. }),
            "an LLM high-risk hint must still force approval regardless of the auto-approve tier");
    }

    #[test]
    fn auto_approve_never_overrides_a_deny() {
        // Denylist deny must win over any auto-approve threshold.
        let mut cfg = autonomous_cfg("shell_exec", ActionRisk::High);
        cfg.denylist = ["shell_exec".to_string()].into_iter().collect();
        assert!(matches!(
            evaluate(&cfg, &req(ActionKind::ShellCommand, "shell_exec", serde_json::json!({"command":"ls"}))),
            PolicyDecision::Deny { .. }
        ));
    }
}
