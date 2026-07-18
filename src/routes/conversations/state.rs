// src/routes/conversations/state.rs
use crate::backend;
use super::workspace_session::WorkspaceSessionRef;
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub struct ConversationsState {
    pub selected_conversation_id: Signal<Option<i64>>,
    pub visible_messages: Signal<Vec<backend::Message>>,
    pub selected_artifacts: Signal<Vec<backend::Artifact>>,
    pub focused_artifact: Signal<Option<backend::Artifact>>,
    pub selected_work_item: Signal<Option<backend::WorkItem>>,
    pub workspace_sessions: Signal<Vec<WorkspaceSessionRef>>,
    pub active_workspace_session_key: Signal<Option<String>>,
    pub starter_prompt: Signal<Option<String>>,
    pub is_thinking: Signal<bool>,
    pub is_local_executing: Signal<bool>,
    pub selected_execution_run_id: Signal<Option<i64>>,
    pub messages_cursor: Signal<Option<i64>>,
    pub messages_has_more: Signal<bool>,
    pub messages_loading_more: Signal<bool>,
}
