use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputerSupervisionState {
    pub supervision_mode: String,
    pub session_state: String,
    pub session_note: String,
    pub allowlist: Vec<String>,
    pub max_actions: i64,
    pub action_count: i64,
    pub timeout_ms: i64,
    pub approval_state: String,
    pub pending_tool: String,
    pub pending_action: String,
    pub pending_domain: String,
    pub session_started_at: Option<String>,
}

fn parse_allowlist(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

pub async fn get_computer_supervision_state(db: &Database) -> ComputerSupervisionState {
    let supervision_mode =
        super::super::get_setting_or(db, "computer_supervision_mode", "approval_required").await;
    let session_state = super::super::get_setting_or(db, "computer_session_state", "idle").await;
    let session_note = super::super::get_setting_or(
        db,
        "computer_session_note",
        "No supervised computer session is active.",
    )
    .await;
    let allowlist =
        parse_allowlist(&super::super::get_setting_or(db, "computer_supervision_allowlist", "").await);
    let max_actions = super::super::get_setting_i64(db, "computer_supervision_max_actions", 25).await;
    let action_count = super::super::get_setting_i64(db, "computer_session_action_count", 0).await;
    let timeout_ms = super::super::get_setting_i64(db, "computer_supervision_session_timeout_ms", 600_000).await;
    let approval_state = super::super::get_setting_or(db, "computer_approval_state", "idle").await;
    let pending_tool = super::super::get_setting_or(db, "computer_pending_tool", "").await;
    let pending_action = super::super::get_setting_or(db, "computer_pending_action", "").await;
    let pending_domain = super::super::get_setting_or(db, "computer_pending_domain", "").await;
    let session_started_at = super::super::get_setting(db, "computer_session_started_at").await;

    ComputerSupervisionState {
        supervision_mode,
        session_state,
        session_note,
        allowlist,
        max_actions,
        action_count,
        timeout_ms,
        approval_state,
        pending_tool,
        pending_action,
        pending_domain,
        session_started_at: if session_started_at.trim().is_empty() {
            None
        } else {
            Some(session_started_at)
        },
    }
}
