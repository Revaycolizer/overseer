// src/routes/conversations/conversations.rs
use super::chat_input::ChatInput;
use super::helpers::copy_to_clipboard;
use super::message_bubble::MessageBubble;
use super::thinking_bubble::ThinkingBubble;
use super::sidebar::Sidebar;
use super::workspace_session::{workspace_session_key, upsert_workspace_session, WorkspaceSessionRef};
use super::workspace_deck::WorkspaceSessionDeck;
use super::registry_panel::RuntimeRegistryPanel;
use super::artifact_panel::RecentArtifactsPanel;
use super::workboard_panel::WorkboardPanel;
use super::empty_state::EmptyConversationState;
use super::route_summary::{derive_execution_route_summary, prettify_status, status_tone};
use crate::backend;
use crate::routes::execution_trace::ExecutionTracePanel;
use dioxus::prelude::document::eval;
use dioxus::prelude::*;

#[component]
fn ChatScreenChip(tone: String, label: String) -> Element {
    let class = match tone.as_str() {
        "blue" => "bg-blue-50 text-blue-700 border-blue-200",
        "violet" => "bg-violet-50 text-violet-700 border-violet-200",
        "green" => "bg-emerald-50 text-emerald-700 border-emerald-200",
        "red" => "bg-red-50 text-red-700 border-red-200",
        _ => "bg-slate-100 text-slate-600 border-slate-200",
    };
    rsx! { span { class: format!("inline-flex items-center rounded-full border px-3 py-1 text-[10px] font-black uppercase tracking-[0.18em] {}", class), "{label}" } }
}

#[component]
pub fn Conversations() -> Element {
    let global_state = crate::state::use_global_state();
    let global_read = global_state.read();
    let db = global_read.db.clone();
    let agent_system = global_read.agent_system.clone();
    drop(global_read);

    let db_for_conv = db.clone();
    let conversations_resource = use_resource(move || { let db = db_for_conv.clone(); async move { backend::get_conversations(&db).await.unwrap_or_default() } });
    let conversations = conversations_resource.read().cloned().unwrap_or_default();
    let mut selected_conversation_id = use_signal(|| conversations.first().map(|c| c.id));

    let db_for_msg = db.clone();
    let messages_resource = use_resource(move || { let db = db_for_msg.clone(); let conv_id = selected_conversation_id(); async move { if let Some(id) = conv_id { backend::get_messages(&db, id).await.unwrap_or_default() } else { vec![] } } });

    let mut visible_messages = use_signal(Vec::<backend::Message>::new);
    let mut selected_conversation_for_cache = use_signal(|| None::<i64>);
    let mut selected_artifacts = use_signal(Vec::<backend::Artifact>::new);
    let mut focused_artifact = use_signal(|| None::<backend::Artifact>);
    let mut selected_work_item = use_signal(|| None::<backend::WorkItem>);
    let mut workspace_sessions = use_signal(Vec::<WorkspaceSessionRef>::new);
    let mut active_workspace_session_key = use_signal(|| None::<String>);

    use_effect(move || { let conv_id = selected_conversation_id(); if selected_conversation_for_cache() != conv_id { selected_conversation_for_cache.set(conv_id); visible_messages.set(Vec::new()); selected_artifacts.set(Vec::new()); focused_artifact.set(None); selected_work_item.set(None); workspace_sessions.set(Vec::new()); active_workspace_session_key.set(None); } });
    use_effect(move || { if let Some(artifact) = focused_artifact() { let key = workspace_sessions.with_mut(|s| upsert_workspace_session(s, WorkspaceSessionRef::Artifact(artifact.id))); active_workspace_session_key.set(Some(key)); } });
    use_effect(move || { if let Some(work_item) = selected_work_item() { let key = workspace_sessions.with_mut(|s| upsert_workspace_session(s, WorkspaceSessionRef::WorkItem(work_item.id))); active_workspace_session_key.set(Some(key)); } });
    use_effect(move || { let sessions = workspace_sessions(); let active_key = active_workspace_session_key(); if sessions.is_empty() { if active_key.is_some() { active_workspace_session_key.set(None); } } else if active_key.as_ref().is_none_or(|key| !sessions.iter().any(|s| workspace_session_key(s) == *key)) { active_workspace_session_key.set(Some(workspace_session_key(&sessions[0]))); } });
    use_effect(move || { if let Some(latest_messages) = messages_resource.read().cloned() { visible_messages.set(latest_messages); } });

    let messages = messages_resource.read().cloned().unwrap_or_else(|| visible_messages());
    let is_first_msg = messages.is_empty();
    let mut starter_prompt = use_signal(|| None::<String>);
    let mut is_thinking: Signal<bool> = use_signal(|| false);
    let mut selected_execution_run_id = use_signal(|| None::<i64>);

    let db_for_execution_runs = db.clone();
    let execution_runs_resource = use_resource(move || { let db = db_for_execution_runs.clone(); let conv_id = selected_conversation_id(); async move { if let Some(id) = conv_id { backend::get_execution_runs_for_conversation(&db, id, 10).await.unwrap_or_default() } else { vec![] } } });
    let execution_runs = execution_runs_resource.read().cloned().unwrap_or_default();
    let runs_loading = execution_runs_resource.read().is_none();

    let execution_runs_for_effect = execution_runs.clone();
    use_effect(move || { let runs = execution_runs_for_effect.clone(); let current = selected_execution_run_id(); if runs.is_empty() { if current.is_some() { selected_execution_run_id.set(None); } } else if !runs.iter().any(|run| Some(run.id) == current) { selected_execution_run_id.set(Some(runs[0].id)); } });

    let db_for_execution_events = db.clone();
    let execution_events_resource = use_resource(move || { let db = db_for_execution_events.clone(); let run_id = selected_execution_run_id(); async move { if let Some(id) = run_id { backend::get_execution_events(&db, id).await.unwrap_or_default() } else { vec![] } } });
    let execution_events = execution_events_resource.read().cloned().unwrap_or_default();
    let events_loading = execution_events_resource.read().is_none();

    let db_for_artifacts = db.clone();
    let artifacts_resource = use_resource(move || { let db = db_for_artifacts.clone(); let conv_id = selected_conversation_id(); async move { if let Some(id) = conv_id { backend::get_artifacts_for_conversation(&db, id, 12).await.unwrap_or_default() } else { vec![] } } });
    let artifacts = artifacts_resource.read().cloned().unwrap_or_default();

    let db_for_work_items = db.clone();
    let work_items_resource = use_resource(move || { let db = db_for_work_items.clone(); let conv_id = selected_conversation_id(); async move { if let Some(id) = conv_id { backend::get_work_items_for_conversation(&db, id, 20).await.unwrap_or_default() } else { vec![] } } });
    let work_items = work_items_resource.read().cloned().unwrap_or_default();

    let db_for_work_item_events = db.clone();
    let work_item_events_resource = use_resource(move || { let db = db_for_work_item_events.clone(); let work_item_id = selected_work_item().map(|i| i.id); async move { if let Some(id) = work_item_id { backend::get_work_item_events(&db, id, 24).await.unwrap_or_default() } else { vec![] } } });
    let work_item_events = work_item_events_resource.read().cloned().unwrap_or_default();

    let db_for_registry_status = db.clone();
    let runtime_registry_resource = use_resource(move || { let db = db_for_registry_status.clone(); async move { backend::get_runtime_registry_status(&db).await } });
    let runtime_registry_status = runtime_registry_resource.read().cloned();

    let selected_conversation = conversations.iter().find(|c| Some(c.id) == selected_conversation_id()).cloned();
    let conversation_title = selected_conversation.as_ref().map(|c| c.title.clone()).unwrap_or_else(|| "New Conversation".to_string());
    let selected_run = execution_runs.iter().find(|r| Some(r.id) == selected_execution_run_id()).cloned();
    let latest_run = execution_runs.first().cloned();
    let (route_label, route_reason, personality_label) = derive_execution_route_summary(&execution_events);

    // Performance: poll frequently only when thinking, otherwise every 10s
    let mut execution_runs_poll = execution_runs_resource.clone();
    let mut execution_events_poll = execution_events_resource.clone();
    let mut artifacts_poll = artifacts_resource.clone();
    let mut work_items_poll = work_items_resource.clone();
    let mut work_item_events_poll = work_item_events_resource.clone();
    let mut runtime_registry_poll = runtime_registry_resource.clone();
    use_future(move || async move {
        loop {
            let interval = if is_thinking() { 2 } else { 10 };
            tokio::time::sleep(std::time::Duration::from_secs(interval)).await;
            execution_runs_poll.restart();
            execution_events_poll.restart();
            artifacts_poll.restart();
            work_items_poll.restart();
            work_item_events_poll.restart();
            runtime_registry_poll.restart();
        }
    });

    use_effect(move || { let _ = messages_resource.read(); let _ = is_thinking(); spawn(async move { tokio::time::sleep(std::time::Duration::from_millis(100)).await; let eval = eval(r#"const container = document.getElementById('chat-scroll-container'); if (container) { const scrollToBottom = (force = false) => { const threshold = 150; const isAtBottom = container.scrollHeight - container.scrollTop - container.clientHeight < threshold; if (force || isAtBottom) { container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' }); } }; scrollToBottom(true); setTimeout(() => scrollToBottom(true), 100); setTimeout(() => scrollToBottom(true), 300); if (!window._overseerScrollObserver) { window._overseerScrollObserver = new MutationObserver(() => { scrollToBottom(); }); window._overseerScrollObserver.observe(container, { childList: true, subtree: true }); } }"#); let _ = eval.await; }); });

    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 min-h-0 relative",
            div { class: "flex flex-1 overflow-hidden",
                Sidebar { conversations: conversations, selected_conversation_id: selected_conversation_id, conversations_resource: conversations_resource.clone(), db: db.clone() }
                main { class: "flex-1 relative bg-[var(--workspace-bg)] overflow-hidden min-h-0",
                    div { class: "h-full grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_380px]",
                        div { class: "flex flex-col min-w-0 min-h-0",
                            div { class: "border-b border-slate-200 bg-white/90 backdrop-blur-sm px-6 py-4",
                                div { class: "max-w-3xl mx-auto flex items-start justify-between gap-4",
                                    div { class: "min-w-0",
                                        p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", if selected_conversation_id().is_some() { "Conversation Workspace" } else { "Ready to Start" } }
                                        h1 { class: "text-lg font-black text-slate-900 truncate mt-1", "{conversation_title}" }
                                        p { class: "text-xs text-slate-500 mt-1 leading-relaxed",
                                            if let Some(reason) = route_reason.clone() { "{reason}" }
                                            else if selected_conversation_id().is_some() { "Send a message to route work through the orchestrator and capture the execution trace." }
                                            else { "Type anything below and Overseer will create a conversation automatically, route the request, and keep the run trace attached." }
                                        }
                                    }
                                    div { class: "flex flex-wrap items-center justify-end gap-2 shrink-0",
                                        if let Some(label) = route_label.clone() { ChatScreenChip { tone: "blue".to_string(), label: label } }
                                        if let Some(label) = personality_label.clone() { ChatScreenChip { tone: "violet".to_string(), label: label } }
                                        if let Some(run) = latest_run.clone() { ChatScreenChip { tone: status_tone(&run.status).to_string(), label: format!("Run {}", prettify_status(&run.status)) } }
                                        else { ChatScreenChip { tone: "slate".to_string(), label: "Trace idle".to_string() } }
                                    }
                                }
                                if let Some(run) = selected_run.clone() {
                                    if let Some(error_message) = run.error_message.clone().filter(|v| !v.trim().is_empty()) {
                                        div { class: "max-w-3xl mx-auto mt-4 rounded-2xl border border-red-200 bg-red-50 px-4 py-3 flex items-start gap-3",
                                            div { class: "mt-0.5 size-6 rounded-full bg-red-100 flex items-center justify-center text-red-600 text-xs font-black", "!" }
                                            div { class: "min-w-0",
                                                p { class: "text-xs font-black uppercase tracking-[0.2em] text-red-600", "Latest Run Failed" }
                                                p { class: "text-xs text-red-700 mt-1 whitespace-pre-wrap break-words", "{error_message}" }
                                            }
                                        }
                                    }
                                }
                            }
                            if !workspace_sessions().is_empty() {
                                div { class: "border-b border-slate-200 bg-white/70 px-6 py-4",
                                    div { class: "max-w-3xl mx-auto",
                                        WorkspaceSessionDeck { db: db.clone(), agent_system: agent_system.clone(), messages_resource: messages_resource.clone(), visible_messages: visible_messages, is_thinking: is_thinking, sessions: workspace_sessions(), sessions_signal: workspace_sessions, active_session_key: active_workspace_session_key, artifacts: artifacts.clone(), work_items: work_items.clone(), selected_artifacts: selected_artifacts, selected_work_item: selected_work_item, starter_prompt: starter_prompt, work_items_resource: work_items_resource.clone(), execution_runs_resource: execution_runs_resource.clone(), artifacts_resource: artifacts_resource.clone(), work_item_events: work_item_events.clone(), work_item_events_resource: work_item_events_resource.clone(), selected_execution_run_id: selected_execution_run_id,
                                            on_open: move |path: String| { spawn(async move { let _ = backend::open_local_path(&path).await; }); },
                                        }
                                    }
                                }
                            }
                            div { id: "chat-scroll-container", class: "flex-1 overflow-y-auto scroll-smooth custom-scrollbar",
                                div { class: "max-w-3xl mx-auto py-10 px-6 space-y-8",
                                    if messages.is_empty() && !is_thinking() {
                                        EmptyConversationState { on_prompt: move |prompt: String| starter_prompt.set(Some(prompt)) }
                                    } else {
                                        for msg in &messages { MessageBubble { msg: msg.clone(), on_copy: move |content: String| copy_to_clipboard(content) } }
                                        if is_thinking() { ThinkingBubble {} }
                                    }
                                }
                            }
                            ChatInput { selected_conversation_id: selected_conversation_id, messages_resource: messages_resource.clone(), visible_messages: visible_messages, conversations_resource: conversations_resource.clone(), selected_artifacts: selected_artifacts, selected_work_item: selected_work_item, is_first_msg: is_first_msg, is_thinking: is_thinking, starter_prompt: starter_prompt }
                        }
                        div { class: "h-full min-h-0 flex flex-col",
                            RuntimeRegistryPanel { status: runtime_registry_status }
                            RecentArtifactsPanel { artifacts: artifacts, selected_artifacts: selected_artifacts, focused_artifact: focused_artifact, on_open: move |path: String| { spawn(async move { let _ = backend::open_local_path(&path).await; }); } }
                            WorkboardPanel { db: db.clone(), agent_system: agent_system.clone(), messages_resource: messages_resource.clone(), visible_messages: visible_messages, work_items: work_items, selected_work_item: selected_work_item, is_thinking: is_thinking, starter_prompt: starter_prompt, work_items_resource: work_items_resource.clone(), execution_runs_resource: execution_runs_resource.clone(), artifacts_resource: artifacts_resource.clone(), work_item_events_resource: work_item_events_resource.clone(), selected_execution_run_id: selected_execution_run_id }
                            div { class: "flex-1 min-h-0",
                                ExecutionTracePanel { heading: "Conversation Trace".to_string(), subtitle: "Inspect the planner, supervisor, tool, and synthesis events behind this conversation.".to_string(), empty_title: "No runs yet".to_string(), empty_body: "Send a message and Overseer will capture execution runs here so you can inspect what happened.".to_string(), runs: execution_runs, events: execution_events, selected_run_id: selected_execution_run_id, runs_loading: runs_loading, events_loading: events_loading }
                            }
                        }
                    }
                }
            }
        }
    }
}
