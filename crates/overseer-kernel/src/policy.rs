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
        }
    }
}

impl PolicyConfig {
    fn is_observation(&self, req: &ActionRequest) -> bool {
        self.observation_tools.contains(&req.tool_name)
    }

    /// A compact, non-sensitive snapshot of the policy for the audit ledger.
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
