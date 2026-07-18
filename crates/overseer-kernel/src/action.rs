//! Typed representation of an action an agent proposes to perform.
//!
//! Every native tool call, Python tool call, shell command or computer-control
//! action is converted into exactly one [`ActionRequest`] before it reaches the
//! kernel. This is the single shared structure the policy engine evaluates.

use serde::{Deserialize, Serialize};

/// The category of side effect an action performs. Used by policy to decide
/// which action kinds require human approval by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    /// A built-in Rust tool (web search, document generation, charting, ...).
    NativeTool,
    /// A user-supplied Python tool executed as a subprocess.
    PythonTool,
    /// A structured OS command (executable + argument array — never `sh -c`).
    ShellCommand,
    /// Mouse / keyboard / screenshot / window control of the desktop.
    ComputerControl,
}

impl ActionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ActionKind::NativeTool => "native_tool",
            ActionKind::PythonTool => "python_tool",
            ActionKind::ShellCommand => "shell_command",
            ActionKind::ComputerControl => "computer_control",
        }
    }
}

/// An immutable description of a single proposed action.
///
/// The kernel evaluates, persists and (on approval) executes *this exact
/// structure*. It is never regenerated after evaluation, so an approved action
/// always executes the payload the human actually saw.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Stable unique id for this action (uuid v4).
    pub id: String,
    /// The execution run this action belongs to.
    pub run_id: i64,
    /// The kind of side effect this action performs.
    pub kind: ActionKind,
    /// The tool / command name being invoked.
    pub tool_name: String,
    /// The exact arguments the tool will receive.
    pub arguments: serde_json::Value,
    /// Capabilities this action needs (usually `[tool_name]`).
    pub requested_capabilities: Vec<String>,
    /// Requested execution timeout in milliseconds.
    pub timeout_ms: u64,
    /// Human-readable one-line summary shown in the approval UI.
    #[serde(default)]
    pub summary: String,
    /// Optional non-authoritative risk hint (e.g. from an LLM heuristic).
    /// Policy MAY use this to *escalate* to approval, but it never grants
    /// authority on its own.
    #[serde(default)]
    pub risk_hint: Option<String>,
}

impl ActionRequest {
    /// Build a request with a freshly generated id.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: i64,
        kind: ActionKind,
        tool_name: impl Into<String>,
        arguments: serde_json::Value,
        requested_capabilities: Vec<String>,
        timeout_ms: u64,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            run_id,
            kind,
            tool_name: tool_name.into(),
            arguments,
            requested_capabilities,
            timeout_ms,
            summary: summary.into(),
            risk_hint: None,
        }
    }

    /// Canonical, immutable serialization of the action payload. This is what is
    /// persisted with the approval request and what is executed on approval, so
    /// the approved action can never be swapped for a different one.
    pub fn canonical_payload(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}
