// src/routes/conversations/content.rs
use crate::backend;
use super::state::ConversationsState;
use super::chat_panel::ChatPanel;
use super::registry_panel::RuntimeRegistryPanel;
use super::artifact_panel::RecentArtifactsPanel;
use super::workboard_panel::WorkboardPanel;
use super::workspace_deck::WorkspaceSessionDeck;
use crate::routes::execution_trace::ExecutionTracePanel;
use dioxus::prelude::*;

#[component]
pub fn ConversationsContent(
    db: std::sync::Arc<crate::db::Database>,
    agent_system: std::sync::Arc<crate::engine::system::AgentSystem>,
    state: ConversationsState,
    messages: Vec<backend::Message>,
    messages_resource: Resource<Vec<backend::Message>>,
    conversations_resource: Resource<Vec<backend::Conversation>>,
    is_first_msg: bool,
    load_more_cb: Callback<(), ()>,
    workspace_sessions: Vec<super::workspace_session::WorkspaceSessionRef>,
    artifacts: Vec<backend::Artifact>,
    work_items: Vec<backend::WorkItem>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    execution_runs_resource: Resource<Vec<backend::ExecutionRun>>,
    artifacts_resource: Resource<Vec<backend::Artifact>>,
    work_item_events: Vec<backend::WorkItemEvent>,
    work_item_events_resource: Resource<Vec<backend::WorkItemEvent>>,
    selected_execution_run_id: Signal<Option<i64>>,
    runtime_registry_status: Option<backend::RuntimeRegistryStatus>,
    execution_runs: Vec<backend::ExecutionRun>,
    execution_events: Vec<backend::ExecutionEvent>,
    runs_loading: bool,
    events_loading: bool,
) -> Element {
    let visible_messages = state.visible_messages;
    let is_thinking = state.is_thinking;
    let selected_artifacts = state.selected_artifacts;
    let selected_work_item = state.selected_work_item;
    let starter_prompt = state.starter_prompt;
    let workspace_sessions_sig = state.workspace_sessions;
    let active_workspace_session_key = state.active_workspace_session_key;

    rsx! {
        div { class: if is_first_msg { "h-full grid grid-cols-1" } else { "h-full grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_380px]" },
            div { class: "flex flex-col min-w-0 min-h-0",
                if !workspace_sessions.is_empty() {
                    div { class: "border-b border-slate-200 bg-white/70 px-6 py-4",
                        div { class: "max-w-3xl mx-auto",
                            WorkspaceSessionDeck { db: db.clone(), agent_system: agent_system.clone(), messages_resource: messages_resource.clone(), visible_messages: visible_messages, is_thinking: is_thinking, sessions: workspace_sessions, sessions_signal: workspace_sessions_sig, active_session_key: active_workspace_session_key, artifacts: artifacts.clone(), work_items: work_items.clone(), selected_artifacts: selected_artifacts, selected_work_item: selected_work_item, starter_prompt: starter_prompt, work_items_resource: work_items_resource.clone(), execution_runs_resource: execution_runs_resource.clone(), artifacts_resource: artifacts_resource.clone(), work_item_events: work_item_events.clone(), work_item_events_resource: work_item_events_resource.clone(), selected_execution_run_id: selected_execution_run_id,
                                on_open: move |path: String| { spawn(async move { let _ = backend::open_local_path(&path).await; }); },
                            }
                        }
                    }
                }
                ChatPanel {
                    messages: messages,
                    messages_resource: messages_resource.clone(),
                    visible_messages: visible_messages,
                    conversations_resource: conversations_resource.clone(),
                    selected_conversation_id: state.selected_conversation_id,
                    selected_artifacts: selected_artifacts,
                    selected_work_item: selected_work_item,
                    is_first_msg: is_first_msg,
                    is_thinking: is_thinking,
                    starter_prompt: starter_prompt,
                    messages_has_more: state.messages_has_more,
                    messages_loading_more: state.messages_loading_more,
                    load_more: load_more_cb,
                }
            }
            if !is_first_msg {
                div { class: "h-full min-h-0 flex flex-col border-l border-slate-200",
                    RuntimeRegistryPanel { status: runtime_registry_status }
                    RecentArtifactsPanel { artifacts: artifacts, selected_artifacts: selected_artifacts, focused_artifact: state.focused_artifact, on_open: move |path: String| { spawn(async move { let _ = backend::open_local_path(&path).await; }); } }
                    WorkboardPanel { db: db.clone(), agent_system: agent_system.clone(), messages_resource: messages_resource.clone(), visible_messages: visible_messages, work_items: work_items, selected_work_item: selected_work_item, is_thinking: is_thinking, starter_prompt: starter_prompt, work_items_resource: work_items_resource.clone(), execution_runs_resource: execution_runs_resource.clone(), artifacts_resource: artifacts_resource.clone(), work_item_events_resource: work_item_events_resource.clone(), selected_execution_run_id: selected_execution_run_id }
                    div { class: "flex-1 min-h-0",
                        ExecutionTracePanel { heading: "Trace".to_string(), subtitle: "".to_string(), empty_title: "No runs yet".to_string(), empty_body: "Runs will appear here once a message is sent.".to_string(), runs: execution_runs, events: execution_events, selected_run_id: selected_execution_run_id, runs_loading: runs_loading, events_loading: events_loading }
                    }
                }
            }
        }
    }
}
