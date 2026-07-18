use super::{sidebar::Sidebar, route_summary::derive_execution_route_summary, layout::ConversationsHeader, content::ConversationsContent, hooks::use_conversations_state, prefetch::setup_prefetching_and_pagination};
use crate::backend;
use dioxus::prelude::document::eval;
use dioxus::prelude::*;

#[component]
pub fn Conversations() -> Element {
    let global_state = crate::state::use_global_state();
    let (db, agent_system) = {
        let r = global_state.read();
        (r.db.clone(), r.agent_system.clone())
    };

    let db_for_conv = db.clone();
    let conversations_resource = use_resource(move || { let db = db_for_conv.clone(); async move { backend::get_conversations(&db, 100).await.unwrap_or_default() } });
    let conversations = conversations_resource.read().cloned().unwrap_or_default();

    let state = use_conversations_state(&conversations);
    let load_more = setup_prefetching_and_pagination(db.clone(), state);

    let db_for_msg = db.clone();
    let selected_conversation_id = state.selected_conversation_id;
    let messages_resource = use_resource(move || { let db = db_for_msg.clone(); let conv_id = selected_conversation_id(); async move { if let Some(id) = conv_id { backend::get_messages_page(&db, id, None, 50).await.unwrap_or_default() } else { vec![] } } });

    let visible = (state.visible_messages)();
    let messages = if visible.is_empty() { messages_resource.read().cloned().unwrap_or_default() } else { visible };
    let is_first_msg = messages.is_empty();

    let db_for_execution_runs = db.clone();
    let execution_runs_resource = use_resource(move || { let db = db_for_execution_runs.clone(); let conv_id = selected_conversation_id(); async move { if let Some(id) = conv_id { backend::get_execution_runs_for_conversation(&db, id, 10).await.unwrap_or_default() } else { vec![] } } });
    provide_context(execution_runs_resource.clone());
    let execution_runs = execution_runs_resource.read().cloned().unwrap_or_default();
    let runs_loading = execution_runs_resource.read().is_none();

    let mut selected_execution_run_id = state.selected_execution_run_id;
    let runs_for_effect = execution_runs.clone();
    use_effect(move || { let runs = runs_for_effect.clone(); let current = selected_execution_run_id(); if runs.is_empty() { if current.is_some() { selected_execution_run_id.set(None); } } else if !runs.iter().any(|run| Some(run.id) == current) { selected_execution_run_id.set(Some(runs[0].id)); } });

    let mut is_thinking = state.is_thinking;
    let is_local_executing = state.is_local_executing;
    provide_context(is_local_executing);
    use_effect(move || {
        if let Some(runs) = execution_runs_resource.read().as_ref() {
            let has_running = runs.iter().any(|run| run.status == "running");
            is_thinking.set(is_local_executing() || has_running);
        } else {
            is_thinking.set(is_local_executing());
        }
    });

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
    let selected_work_item = state.selected_work_item;
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

    use_effect(move || { let _ = messages_resource.read(); let _ = is_thinking(); spawn(async move { tokio::time::sleep(std::time::Duration::from_millis(100)).await; let eval = eval(r#"const container = document.getElementById('chat-scroll-container'); if (container) { const scrollToBottom = (force = false) => { const threshold = 150; const isAtBottom = container.scrollHeight - container.scrollTop - container.clientHeight < threshold; if (force || isAtBottom) { container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' }); } }; scrollToBottom(true); setTimeout(() => scrollToBottom(true), 100); setTimeout(() => scrollToBottom(true), 300); if (!window._overseerScrollObserver) { window._overseerScrollObserver = new MutationObserver(() => { scrollToBottom(); }); window._overseerScrollObserver.observe(container, { childList: true, subtree: true }); } }"#); let _ = eval.await; }); });

    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 min-h-0 relative",
            div { class: "flex flex-1 overflow-hidden",
                Sidebar { conversations: conversations, selected_conversation_id: selected_conversation_id, conversations_resource: conversations_resource.clone(), db: db.clone() }
                main { class: "flex-1 relative bg-[var(--workspace-bg)] overflow-hidden min-h-0",
                    div { class: "flex flex-col min-w-0 min-h-0 h-full",
                        ConversationsHeader { state: state, conversation_title: conversation_title, route_reason: route_reason, route_label: route_label, personality_label: personality_label, latest_run: latest_run, selected_run: selected_run }
                        ConversationsContent { db: db.clone(), agent_system: agent_system.clone(), state: state, messages: messages, messages_resource: messages_resource, conversations_resource: conversations_resource, is_first_msg: is_first_msg, load_more_cb: Callback::new(move |_| load_more()), workspace_sessions: (state.workspace_sessions)(), artifacts: artifacts, work_items: work_items, work_items_resource: work_items_resource, execution_runs_resource: execution_runs_resource, artifacts_resource: artifacts_resource, work_item_events: work_item_events, work_item_events_resource: work_item_events_resource, selected_execution_run_id: selected_execution_run_id, runtime_registry_status: runtime_registry_status, execution_runs: execution_runs, execution_events: execution_events, runs_loading: runs_loading, events_loading: events_loading }
                    }
                }
            }
        }
    }
}
