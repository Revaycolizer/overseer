// src/routes/conversations/conversations.rs
use dioxus::prelude::*;
use dioxus::prelude::document::eval;
use crate::backend;
use crate::routes::execution_trace::ExecutionTracePanel;
use super::message_bubble::{MessageBubble, ThinkingBubble};
use super::helpers::copy_to_clipboard;
use super::sidebar::Sidebar;
use super::chat_input::ChatInput;

#[component]
pub fn Conversations() -> Element {
    let global_state = crate::state::use_global_state();
    let global_read = global_state.read();
    let db = global_read.db.clone();
    drop(global_read);

    let db_for_conv = db.clone();
    let conversations_resource = use_resource(move || {
        let db = db_for_conv.clone();
        async move { backend::get_conversations(&db).await.unwrap_or_default() }
    });

    let conversations = conversations_resource.read().cloned().unwrap_or_default();
    let mut selected_conversation_id =
        use_signal(|| conversations.first().map(|c| c.id));

    let db_for_msg = db.clone();
    let messages_resource = use_resource(move || {
        let db = db_for_msg.clone();
        let conv_id = selected_conversation_id();
        async move {
            if let Some(id) = conv_id {
                backend::get_messages(&db, id).await.unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let messages = messages_resource.read().cloned().unwrap_or_default();
    let is_first_msg = messages.is_empty();
    let mut is_thinking: Signal<bool> = use_signal(|| false);
    let mut selected_execution_run_id = use_signal(|| None::<i64>);

    let db_for_execution_runs = db.clone();
    let execution_runs_resource = use_resource(move || {
        let db = db_for_execution_runs.clone();
        let conv_id = selected_conversation_id();
        async move {
            if let Some(id) = conv_id {
                backend::get_execution_runs_for_conversation(&db, id, 10).await.unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let execution_runs = execution_runs_resource.read().cloned().unwrap_or_default();
    let runs_loading = execution_runs_resource.read().is_none();

    let execution_runs_for_effect = execution_runs.clone();
    use_effect(move || {
        let runs = execution_runs_for_effect.clone();
        let current = selected_execution_run_id();
        if runs.is_empty() {
            if current.is_some() {
                selected_execution_run_id.set(None);
            }
        } else if !runs.iter().any(|run| Some(run.id) == current) {
            selected_execution_run_id.set(Some(runs[0].id));
        }
    });

    let db_for_execution_events = db.clone();
    let execution_events_resource = use_resource(move || {
        let db = db_for_execution_events.clone();
        let run_id = selected_execution_run_id();
        async move {
            if let Some(id) = run_id {
                backend::get_execution_events(&db, id).await.unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let execution_events = execution_events_resource.read().cloned().unwrap_or_default();
    let events_loading = execution_events_resource.read().is_none();

    let mut execution_runs_poll = execution_runs_resource.clone();
    let mut execution_events_poll = execution_events_resource.clone();
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            execution_runs_poll.restart();
            execution_events_poll.restart();
        }
    });

    // Auto-scroll to bottom when messages change
    use_effect(move || {
        let _ = messages_resource.read();
        let _ = is_thinking();
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let eval = eval(r#"
                const container = document.getElementById('chat-scroll-container');
                if (container) {
                    const scrollToBottom = (force = false) => {
                        const threshold = 150;
                        const isAtBottom =
                            container.scrollHeight - container.scrollTop - container.clientHeight < threshold;
                        if (force || isAtBottom) {
                            container.scrollTo({ top: container.scrollHeight, behavior: 'smooth' });
                        }
                    };
                    scrollToBottom(true);
                    setTimeout(() => scrollToBottom(true), 100);
                    setTimeout(() => scrollToBottom(true), 300);
                    if (!window._overseerScrollObserver) {
                        window._overseerScrollObserver = new MutationObserver(() => {
                            scrollToBottom();
                        });
                        window._overseerScrollObserver.observe(container, {
                            childList: true,
                            subtree: true
                        });
                    }
                }
            "#);
            let _ = eval.await;
        });
    });

    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 min-h-0 relative",
            div { class: "flex flex-1 overflow-hidden",
                Sidebar {
                    conversations: conversations,
                    selected_conversation_id: selected_conversation_id,
                    conversations_resource: conversations_resource.clone(),
                    db: db.clone(),
                }
                main { class: "flex-1 relative bg-[var(--workspace-bg)] overflow-hidden min-h-0",
                    div { class: "h-full grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_380px]",
                        div { class: "flex flex-col min-w-0 min-h-0",
                            div {
                                id: "chat-scroll-container",
                                class: "flex-1 overflow-y-auto scroll-smooth custom-scrollbar",
                                div { class: "max-w-3xl mx-auto py-10 px-6 space-y-8",
                                    for msg in &messages {
                                        MessageBubble {
                                            msg: msg.clone(),
                                            on_copy: move |content: String| copy_to_clipboard(content)
                                        }
                                    }
                                    if is_thinking() {
                                        ThinkingBubble {}
                                    }
                                }
                            }
                            ChatInput {
                                selected_conversation_id: selected_conversation_id,
                                messages_resource: messages_resource.clone(),
                                conversations_resource: conversations_resource.clone(),
                                is_first_msg: is_first_msg,
                                is_thinking: is_thinking,
                            }
                        }
                        ExecutionTracePanel {
                            heading: "Conversation Trace".to_string(),
                            subtitle: "Inspect the planner, supervisor, tool, and synthesis events behind this conversation.".to_string(),
                            empty_title: "No runs yet".to_string(),
                            empty_body: "Send a message and Overseer will capture execution runs here so you can inspect what happened.".to_string(),
                            runs: execution_runs,
                            events: execution_events,
                            selected_run_id: selected_execution_run_id,
                            runs_loading: runs_loading,
                            events_loading: events_loading,
                        }
                    }
                }
            }
        }
    }
}
