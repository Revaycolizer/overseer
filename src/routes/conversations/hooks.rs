// src/routes/conversations/hooks.rs
use crate::backend;
use super::state::ConversationsState;
use super::workspace_session::{upsert_workspace_session, WorkspaceSessionRef};
use dioxus::prelude::*;

pub fn use_conversations_state(
    conversations: &[backend::Conversation],
) -> ConversationsState {
    let selected_conversation_id = use_signal(|| conversations.first().map(|c| c.id));
    let mut visible_messages = use_signal(Vec::<backend::Message>::new);
    let mut selected_conversation_for_cache = use_signal(|| None::<i64>);
    let mut selected_artifacts = use_signal(Vec::<backend::Artifact>::new);
    let mut focused_artifact = use_signal(|| None::<backend::Artifact>);
    let mut selected_work_item = use_signal(|| None::<backend::WorkItem>);
    let mut workspace_sessions = use_signal(Vec::<WorkspaceSessionRef>::new);
    let mut active_workspace_session_key = use_signal(|| None::<String>);

    use_effect(move || {
        let conv_id = selected_conversation_id();
        if selected_conversation_for_cache() != conv_id {
            selected_conversation_for_cache.set(conv_id);
            visible_messages.set(Vec::new());
            selected_artifacts.set(Vec::new());
            focused_artifact.set(None);
            selected_work_item.set(None);
            workspace_sessions.set(Vec::new());
            active_workspace_session_key.set(None);
        }
    });

    use_effect(move || {
        if let Some(artifact) = focused_artifact() {
            let key = workspace_sessions.with_mut(|s| upsert_workspace_session(s, WorkspaceSessionRef::Artifact(artifact.id)));
            active_workspace_session_key.set(Some(key));
        }
    });

    use_effect(move || {
        if let Some(work_item) = selected_work_item() {
            let key = workspace_sessions.with_mut(|s| upsert_workspace_session(s, WorkspaceSessionRef::WorkItem(work_item.id)));
            active_workspace_session_key.set(Some(key));
        }
    });

    use_effect(move || {
        let sessions = workspace_sessions();
        let active_key = active_workspace_session_key();
        if sessions.is_empty() {
            if active_key.is_some() {
                active_workspace_session_key.set(None);
            }
        } else if active_key.as_ref().is_none_or(|key| !sessions.iter().any(|s| super::workspace_session::workspace_session_key(s) == *key)) {
            active_workspace_session_key.set(Some(super::workspace_session::workspace_session_key(&sessions[0])));
        }
    });

    ConversationsState {
        selected_conversation_id,
        visible_messages,
        selected_artifacts,
        focused_artifact,
        selected_work_item,
        workspace_sessions,
        active_workspace_session_key,
        starter_prompt: use_signal(|| None::<String>),
        is_thinking: use_signal(|| false),
        is_local_executing: use_signal(|| false),
        selected_execution_run_id: use_signal(|| None::<i64>),
        messages_cursor: use_signal(|| None),
        messages_has_more: use_signal(|| true),
        messages_loading_more: use_signal(|| false),
    }
}
