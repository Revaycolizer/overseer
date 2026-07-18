//! Structured kernel trace events and the sink they are written through.
//!
//! The kernel never talks to the database directly for tracing. It emits
//! [`KernelEvent`]s to a [`TraceSink`], which the host application implements
//! (in Overseer, over the existing `execution_events` table). Tests use an
//! in-memory sink.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// The lifecycle events the kernel records for every action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum KernelEventKind {
    ActionRequested,
    PolicyAllowed,
    PolicyDenied,
    ApprovalRequested,
    ApprovalApproved,
    ApprovalRejected,
    ApprovalCancelled,
    ActionStarted,
    ActionCompleted,
    ActionFailed,
    ActionTimedOut,
    EmergencyStop,
}

impl KernelEventKind {
    /// The `event_type` string stored in the trace log.
    pub fn as_str(&self) -> &'static str {
        match self {
            KernelEventKind::ActionRequested => "action_requested",
            KernelEventKind::PolicyAllowed => "policy_allowed",
            KernelEventKind::PolicyDenied => "policy_denied",
            KernelEventKind::ApprovalRequested => "approval_requested",
            KernelEventKind::ApprovalApproved => "approval_approved",
            KernelEventKind::ApprovalRejected => "approval_rejected",
            KernelEventKind::ApprovalCancelled => "approval_cancelled",
            KernelEventKind::ActionStarted => "action_started",
            KernelEventKind::ActionCompleted => "action_completed",
            KernelEventKind::ActionFailed => "action_failed",
            KernelEventKind::ActionTimedOut => "action_timed_out",
            KernelEventKind::EmergencyStop => "emergency_stop",
        }
    }

    /// A reasonable trace `status` for this event kind.
    pub fn status(&self) -> &'static str {
        match self {
            KernelEventKind::ActionRequested | KernelEventKind::ActionStarted => "running",
            KernelEventKind::PolicyAllowed
            | KernelEventKind::ApprovalRequested
            | KernelEventKind::ApprovalApproved => "info",
            KernelEventKind::ActionCompleted => "completed",
            KernelEventKind::PolicyDenied
            | KernelEventKind::ApprovalRejected
            | KernelEventKind::ApprovalCancelled
            | KernelEventKind::ActionFailed
            | KernelEventKind::ActionTimedOut
            | KernelEventKind::EmergencyStop => "failed",
        }
    }
}

/// One structured event about one action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KernelEvent {
    pub run_id: i64,
    pub kind: KernelEventKind,
    pub action_id: String,
    pub tool_name: String,
    /// Human-readable message. Must not contain secrets.
    pub message: String,
    /// Optional structured JSON detail. Must not contain secrets or full
    /// sensitive environment values.
    pub details: Option<String>,
}

impl KernelEvent {
    pub fn new(
        run_id: i64,
        kind: KernelEventKind,
        action_id: impl Into<String>,
        tool_name: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            run_id,
            kind,
            action_id: action_id.into(),
            tool_name: tool_name.into(),
            message: message.into(),
            details: None,
        }
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }
}

/// Where kernel events are recorded. Implemented by the host application.
#[async_trait]
pub trait TraceSink: Send + Sync {
    async fn record(&self, event: KernelEvent);
}
