// src/routes/conversations/conversations.rs
use super::chat_input::ChatInput;
use super::helpers::copy_to_clipboard;
use super::message_bubble::{MessageBubble, ThinkingBubble};
use super::sidebar::Sidebar;
use crate::backend;
use crate::routes::execution_trace::ExecutionTracePanel;
use base64::{engine::general_purpose, Engine as _};
use dioxus::prelude::document::eval;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use serde_json::Value;

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
    let mut selected_conversation_id = use_signal(|| conversations.first().map(|c| c.id));

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

    let mut visible_messages = use_signal(Vec::<backend::Message>::new);
    let mut selected_conversation_for_cache = use_signal(|| None::<i64>);
    let mut selected_artifacts = use_signal(Vec::<backend::Artifact>::new);
    let mut focused_artifact = use_signal(|| None::<backend::Artifact>);
    let mut selected_work_item = use_signal(|| None::<backend::WorkItem>);

    use_effect(move || {
        let conv_id = selected_conversation_id();
        if selected_conversation_for_cache() != conv_id {
            selected_conversation_for_cache.set(conv_id);
            visible_messages.set(Vec::new());
            selected_artifacts.set(Vec::new());
            focused_artifact.set(None);
            selected_work_item.set(None);
        }
    });

    use_effect(move || {
        if let Some(latest_messages) = messages_resource.read().cloned() {
            visible_messages.set(latest_messages);
        }
    });

    let messages = messages_resource
        .read()
        .cloned()
        .unwrap_or_else(|| visible_messages());
    let is_first_msg = messages.is_empty();
    let mut starter_prompt = use_signal(|| None::<String>);
    let mut is_thinking: Signal<bool> = use_signal(|| false);
    let mut selected_execution_run_id = use_signal(|| None::<i64>);

    let db_for_execution_runs = db.clone();
    let execution_runs_resource = use_resource(move || {
        let db = db_for_execution_runs.clone();
        let conv_id = selected_conversation_id();
        async move {
            if let Some(id) = conv_id {
                backend::get_execution_runs_for_conversation(&db, id, 10)
                    .await
                    .unwrap_or_default()
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
                backend::get_execution_events(&db, id)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let execution_events = execution_events_resource
        .read()
        .cloned()
        .unwrap_or_default();
    let events_loading = execution_events_resource.read().is_none();

    let db_for_artifacts = db.clone();
    let artifacts_resource = use_resource(move || {
        let db = db_for_artifacts.clone();
        let conv_id = selected_conversation_id();
        async move {
            if let Some(id) = conv_id {
                backend::get_artifacts_for_conversation(&db, id, 12)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let artifacts = artifacts_resource.read().cloned().unwrap_or_default();
    let db_for_work_items = db.clone();
    let work_items_resource = use_resource(move || {
        let db = db_for_work_items.clone();
        let conv_id = selected_conversation_id();
        async move {
            if let Some(id) = conv_id {
                backend::get_work_items_for_conversation(&db, id, 20)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });
    let work_items = work_items_resource.read().cloned().unwrap_or_default();
    let selected_conversation = conversations
        .iter()
        .find(|conversation| Some(conversation.id) == selected_conversation_id())
        .cloned();
    let conversation_title = selected_conversation
        .as_ref()
        .map(|conversation| conversation.title.clone())
        .unwrap_or_else(|| "New Conversation".to_string());
    let selected_run = execution_runs
        .iter()
        .find(|run| Some(run.id) == selected_execution_run_id())
        .cloned();
    let latest_run = execution_runs.first().cloned();
    let (route_label, route_reason, personality_label) =
        derive_execution_route_summary(&execution_events);

    let mut execution_runs_poll = execution_runs_resource.clone();
    let mut execution_events_poll = execution_events_resource.clone();
    let mut artifacts_poll = artifacts_resource.clone();
    let mut work_items_poll = work_items_resource.clone();
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            execution_runs_poll.restart();
            execution_events_poll.restart();
            artifacts_poll.restart();
            work_items_poll.restart();
        }
    });

    // Auto-scroll to bottom when messages change
    use_effect(move || {
        let _ = messages_resource.read();
        let _ = is_thinking();
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            let eval = eval(
                r#"
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
            "#,
            );
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
                            div { class: "border-b border-slate-200 bg-white/90 backdrop-blur-sm px-6 py-4",
                                div { class: "max-w-3xl mx-auto flex items-start justify-between gap-4",
                                    div { class: "min-w-0",
                                        p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400",
                                            if selected_conversation_id().is_some() {
                                                "Conversation Workspace"
                                            } else {
                                                "Ready to Start"
                                            }
                                        }
                                        h1 { class: "text-lg font-black text-slate-900 truncate mt-1",
                                            "{conversation_title}"
                                        }
                                        p { class: "text-xs text-slate-500 mt-1 leading-relaxed",
                                            if let Some(reason) = route_reason.clone() {
                                                "{reason}"
                                            } else if selected_conversation_id().is_some() {
                                                "Send a message to route work through the orchestrator and capture the execution trace."
                                            } else {
                                                "Type anything below and Overseer will create a conversation automatically, route the request, and keep the run trace attached."
                                            }
                                        }
                                    }
                                    div { class: "flex flex-wrap items-center justify-end gap-2 shrink-0",
                                        if let Some(label) = route_label.clone() {
                                            ChatScreenChip {
                                                tone: "blue".to_string(),
                                                label: label,
                                            }
                                        }
                                        if let Some(label) = personality_label.clone() {
                                            ChatScreenChip {
                                                tone: "violet".to_string(),
                                                label: label,
                                            }
                                        }
                                        if let Some(run) = latest_run.clone() {
                                            ChatScreenChip {
                                                tone: status_tone(&run.status).to_string(),
                                                label: format!("Run {}", prettify_status(&run.status)),
                                            }
                                        } else {
                                            ChatScreenChip {
                                                tone: "slate".to_string(),
                                                label: "Trace idle".to_string(),
                                            }
                                        }
                                    }
                                }
                                if let Some(run) = selected_run.clone() {
                                    if let Some(error_message) = run.error_message.clone().filter(|value| !value.trim().is_empty()) {
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
                            div {
                                id: "chat-scroll-container",
                                class: "flex-1 overflow-y-auto scroll-smooth custom-scrollbar",
                                div { class: "max-w-3xl mx-auto py-10 px-6 space-y-8",
                                    if messages.is_empty() && !is_thinking() {
                                        EmptyConversationState {
                                            on_prompt: move |prompt: String| starter_prompt.set(Some(prompt)),
                                        }
                                    } else {
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
                            }
                            ChatInput {
                                selected_conversation_id: selected_conversation_id,
                                messages_resource: messages_resource.clone(),
                                visible_messages: visible_messages,
                                conversations_resource: conversations_resource.clone(),
                                selected_artifacts: selected_artifacts,
                                selected_work_item: selected_work_item,
                                is_first_msg: is_first_msg,
                                is_thinking: is_thinking,
                                starter_prompt: starter_prompt,
                            }
                        }
                        div { class: "h-full min-h-0 flex flex-col",
                            RecentArtifactsPanel {
                                db: db.clone(),
                                artifacts: artifacts,
                                selected_artifacts: selected_artifacts,
                                focused_artifact: focused_artifact,
                                selected_work_item: selected_work_item,
                                starter_prompt: starter_prompt,
                                work_items_resource: work_items_resource.clone(),
                                on_open: move |path: String| {
                                    spawn(async move {
                                        let _ = backend::open_local_path(&path).await;
                                    });
                                },
                            }
                            WorkboardPanel {
                                db: db.clone(),
                                work_items: work_items,
                                selected_work_item: selected_work_item,
                                starter_prompt: starter_prompt,
                                work_items_resource: work_items_resource.clone(),
                            }
                            div { class: "flex-1 min-h-0",
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
    }
}

fn derive_execution_route_summary(
    events: &[backend::ExecutionEvent],
) -> (Option<String>, Option<String>, Option<String>) {
    let decision = events
        .iter()
        .find(|event| event.event_type == "orchestration_decision");

    let Some(event) = decision else {
        return (None, None, None);
    };

    let details = event
        .details
        .as_deref()
        .and_then(|details| serde_json::from_str::<Value>(details).ok());

    let route = details
        .as_ref()
        .and_then(|value| value.get("route"))
        .and_then(|value| value.as_str())
        .map(prettify_route);

    let domain = details
        .as_ref()
        .and_then(|value| value.get("domain"))
        .and_then(|value| value.as_str())
        .filter(|value| !value.trim().is_empty() && *value != "null")
        .map(prettify_token);

    let personality = details
        .as_ref()
        .and_then(|value| value.get("personality_mode"))
        .and_then(|value| value.as_str())
        .map(prettify_token);

    let reason = details
        .as_ref()
        .and_then(|value| value.get("reason"))
        .and_then(|value| value.as_str())
        .map(|value| value.to_string())
        .or_else(|| event.message.clone());

    let route_label = match (route, domain) {
        (Some(route), Some(domain)) if !route.contains(&domain) => {
            Some(format!("{route}: {domain}"))
        }
        (Some(route), _) => Some(route),
        _ => None,
    };

    (route_label, reason, personality)
}

fn prettify_route(value: &str) -> String {
    match value {
        "direct_chat" => "Direct Chat".to_string(),
        "single_domain" => "Single Domain".to_string(),
        "multi_step" => "Multi-Step".to_string(),
        other => prettify_token(other),
    }
}

fn prettify_status(value: &str) -> String {
    prettify_token(value)
}

fn prettify_token(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn status_tone(status: &str) -> &'static str {
    match status {
        "completed" | "success" => "green",
        "failed" | "error" => "red",
        "running" | "in_progress" => "blue",
        _ => "slate",
    }
}

#[component]
fn RecentArtifactsPanel(
    db: std::sync::Arc<crate::db::Database>,
    artifacts: Vec<backend::Artifact>,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    focused_artifact: Signal<Option<backend::Artifact>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    on_open: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "border-l border-slate-200 border-b border-slate-200 bg-white/95 backdrop-blur-sm",
            div { class: "px-5 py-4 space-y-3",
                div {
                    p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Recent Artifacts" }
                    h3 { class: "text-sm font-black text-slate-900 mt-1", "Outputs that agents can hand off, inspect, or reuse." }
                }

                if artifacts.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-slate-200 bg-slate-50 px-4 py-3 text-xs text-slate-500 leading-relaxed",
                        "No artifacts recorded yet. Screenshot files, links, and machine-state outputs will show up here after a run produces them."
                    }
                } else {
                    div { class: "space-y-2 max-h-72 overflow-y-auto custom-scrollbar pr-1",
                        for artifact in artifacts {
                            ArtifactCard {
                                key: "{artifact.id}",
                                artifact: artifact,
                                selected_artifacts: selected_artifacts,
                                focused_artifact: focused_artifact,
                                on_open: on_open.clone(),
                            }
                        }
                    }
                }

                if let Some(artifact) = focused_artifact() {
                    ArtifactWorkbenchPanel {
                        db: db,
                        artifact: artifact,
                        selected_artifacts: selected_artifacts,
                        selected_work_item: selected_work_item,
                        starter_prompt: starter_prompt,
                        work_items_resource: work_items_resource,
                        on_open: on_open,
                    }
                }
            }
        }
    }
}

#[component]
fn ArtifactKindBadge(kind: String) -> Element {
    let (label, class) = match kind.as_str() {
        "image" => ("Image", "bg-blue-50 text-blue-700 border-blue-200"),
        "file" => ("File", "bg-emerald-50 text-emerald-700 border-emerald-200"),
        "link" => ("Link", "bg-violet-50 text-violet-700 border-violet-200"),
        "record" => ("Record", "bg-amber-50 text-amber-700 border-amber-200"),
        _ => ("Note", "bg-slate-100 text-slate-600 border-slate-200"),
    };

    rsx! {
        span { class: format!("inline-flex items-center rounded-full border px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] {}", class),
            "{label}"
        }
    }
}

#[component]
fn ArtifactCard(
    artifact: backend::Artifact,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    focused_artifact: Signal<Option<backend::Artifact>>,
    on_open: EventHandler<String>,
) -> Element {
    let artifact_kind = artifact.kind.clone();
    let artifact_content_type = artifact.content_type.clone();
    let file_path = artifact.file_path.clone();
    let is_selected = selected_artifacts()
        .iter()
        .any(|selected| selected.id == artifact.id);
    let is_focused = focused_artifact()
        .as_ref()
        .map(|focused| focused.id == artifact.id)
        .unwrap_or(false);
    let image_preview = use_resource(move || {
        let kind = artifact_kind.clone();
        let file_path = file_path.clone();
        let content_type = artifact_content_type
            .clone()
            .unwrap_or_else(|| "image/png".to_string());
        async move {
            if kind != "image" {
                return None;
            }
            let Some(path) = file_path else {
                return None;
            };
            match std::fs::read(&path) {
                Ok(bytes) if !bytes.is_empty() => Some(format!(
                    "data:{};base64,{}",
                    content_type,
                    general_purpose::STANDARD.encode(bytes)
                )),
                _ => None,
            }
        }
    });

    rsx! {
        div { class: format!(
                "rounded-2xl border px-4 py-3 space-y-3 transition-all {}",
                if is_focused {
                    "border-blue-200 bg-blue-50/60 shadow-sm"
                } else {
                    "border-slate-200 bg-slate-50/80"
                }
            ),
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0",
                    p { class: "text-xs font-black text-slate-900 truncate", "{artifact.label}" }
                    p { class: "text-[11px] text-slate-400 mt-1",
                        {artifact.created_at.format("%Y-%m-%d %H:%M").to_string()}
                    }
                }
                ArtifactKindBadge { kind: artifact.kind.clone() }
            }

            button {
                class: format!(
                    "inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] transition-all {}",
                    if is_selected {
                        "border-blue-200 bg-blue-50 text-blue-700"
                    } else {
                        "border-slate-200 bg-white text-slate-600 hover:border-blue-200 hover:text-blue-700"
                    }
                ),
                onclick: {
                    let artifact = artifact.clone();
                    move |_| {
                        selected_artifacts.with_mut(|selected| {
                            if let Some(index) =
                                selected.iter().position(|item| item.id == artifact.id)
                            {
                                selected.remove(index);
                            } else {
                                selected.push(artifact.clone());
                            }
                        });
                    }
                },
                MaterialIcon { name: if is_selected { "task_alt" } else { "add_circle" }, size: 14 }
                if is_selected {
                    "Selected for Next Prompt"
                } else {
                    "Use in Next Prompt"
                }
            }

            button {
                class: format!(
                    "inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] transition-all {}",
                    if is_focused {
                        "border-blue-200 bg-white text-blue-700"
                    } else {
                        "border-slate-200 bg-white text-slate-600 hover:border-blue-200 hover:text-blue-700"
                    }
                ),
                onclick: {
                    let artifact = artifact.clone();
                    move |_| focused_artifact.set(Some(artifact.clone()))
                },
                MaterialIcon { name: "visibility", size: 14 }
                if is_focused {
                    "Inspector Open"
                } else {
                    "Inspect"
                }
            }

            if let Some(domain) = artifact.domain.clone().filter(|value| !value.trim().is_empty()) {
                p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400",
                    "{prettify_token(&domain)}"
                }
            }
            if let Some(step_text) = artifact.step_text.clone().filter(|value| !value.trim().is_empty()) {
                p { class: "text-xs text-slate-600 line-clamp-2", "{step_text}" }
            }

            if artifact.kind == "image" {
                match image_preview().flatten() {
                    Some(src) => rsx! {
                        img {
                            src: "{src}",
                            alt: "{artifact.label}",
                            class: "rounded-xl border border-slate-200 bg-white shadow-sm max-w-full h-auto block"
                        }
                    },
                    None => rsx! {
                        div { class: "rounded-xl border border-dashed border-slate-200 bg-white px-4 py-5 text-xs text-slate-400",
                            "Image preview unavailable."
                        }
                    },
                }
            }

            if let Some(path) = artifact.file_path.clone().filter(|value| !value.trim().is_empty()) {
                p { class: "text-[11px] font-mono text-slate-500 break-all", "{path}" }
                div { class: "flex flex-wrap gap-2",
                    button {
                        class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] text-slate-600 hover:border-blue-200 hover:text-blue-700 transition-all",
                        onclick: {
                            let path = path.clone();
                            move |_| on_open.call(path.clone())
                        },
                        MaterialIcon { name: "open_in_new", size: 14 }
                        "Open"
                    }
                    button {
                        class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] text-slate-600 hover:border-blue-200 hover:text-blue-700 transition-all",
                        onclick: {
                            let path = path.clone();
                            move |_| copy_to_clipboard(path.clone())
                        },
                        MaterialIcon { name: "content_copy", size: 14 }
                        "Copy Path"
                    }
                }
            } else if let Some(content) = artifact.content.clone().filter(|value| !value.trim().is_empty()) {
                p { class: "text-xs text-slate-600 whitespace-pre-wrap break-words line-clamp-3", "{content}" }
                button {
                    class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] text-slate-600 hover:border-blue-200 hover:text-blue-700 transition-all",
                    onclick: {
                        let content = content.clone();
                        move |_| copy_to_clipboard(content.clone())
                    },
                    MaterialIcon { name: "content_copy", size: 14 }
                    "Copy"
                }
            }
        }
    }
}

#[component]
fn ArtifactWorkbenchPanel(
    db: std::sync::Arc<crate::db::Database>,
    artifact: backend::Artifact,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    on_open: EventHandler<String>,
) -> Element {
    let artifact_kind = artifact.kind.clone();
    let artifact_content_type = artifact.content_type.clone();
    let file_path = artifact.file_path.clone();
    let image_preview = use_resource(move || {
        let kind = artifact_kind.clone();
        let file_path = file_path.clone();
        let content_type = artifact_content_type
            .clone()
            .unwrap_or_else(|| "image/png".to_string());
        async move {
            if kind != "image" {
                return None;
            }
            let Some(path) = file_path else {
                return None;
            };
            match std::fs::read(&path) {
                Ok(bytes) if !bytes.is_empty() => Some(format!(
                    "data:{};base64,{}",
                    content_type,
                    general_purpose::STANDARD.encode(bytes)
                )),
                _ => None,
            }
        }
    });
    let is_selected = selected_artifacts()
        .iter()
        .any(|selected| selected.id == artifact.id);
    let has_other_selected = selected_artifacts()
        .iter()
        .any(|selected| selected.id != artifact.id);
    let other_selected_labels = selected_artifacts()
        .iter()
        .filter(|selected| selected.id != artifact.id)
        .map(|selected| selected.label.clone())
        .collect::<Vec<_>>();
    let default_surface = if has_other_selected {
        "compare".to_string()
    } else {
        "summary".to_string()
    };
    let mut active_surface = use_signal(move || default_surface.clone());

    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-gradient-to-br from-slate-50 to-white px-4 py-4 shadow-sm space-y-4",
            div {
                p { class: "text-[10px] font-black uppercase tracking-[0.2em] text-slate-400", "Artifact Inspector" }
                h4 { class: "text-sm font-black text-slate-900 mt-1", "{artifact.label}" }
                p { class: "text-xs text-slate-500 mt-1 leading-relaxed",
                    "Inspect this artifact, reuse it in the next prompt, or turn it into a more targeted task."
                }
            }

            div { class: "flex flex-wrap items-center gap-2",
                ArtifactKindBadge { kind: artifact.kind.clone() }
                if let Some(domain) = artifact.domain.clone().filter(|value| !value.trim().is_empty()) {
                    span { class: "inline-flex items-center rounded-full border border-slate-200 bg-white px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-slate-500",
                        "{prettify_token(&domain)}"
                    }
                }
                span { class: "text-[10px] text-slate-400",
                    {artifact.created_at.format("%Y-%m-%d %H:%M").to_string()}
                }
            }

            if artifact.kind == "image" {
                match image_preview().flatten() {
                    Some(src) => rsx! {
                        img {
                            src: "{src}",
                            alt: "{artifact.label}",
                            class: "rounded-xl border border-slate-200 bg-white shadow-sm max-w-full h-auto block"
                        }
                    },
                    None => rsx! {
                        div { class: "rounded-xl border border-dashed border-slate-200 bg-white px-4 py-5 text-xs text-slate-400",
                            "Image preview unavailable."
                        }
                    },
                }
            }

            if let Some(step_text) = artifact.step_text.clone().filter(|value| !value.trim().is_empty()) {
                p { class: "text-xs text-slate-600 leading-relaxed", "{step_text}" }
            }

            div { class: "space-y-3",
                div { class: "flex flex-wrap gap-2",
                    ArtifactSurfaceModeButton {
                        label: "Summary".to_string(),
                        icon: "summarize".to_string(),
                        value: "summary".to_string(),
                        active_surface: active_surface,
                    }
                    ArtifactSurfaceModeButton {
                        label: "Review".to_string(),
                        icon: "fact_check".to_string(),
                        value: "review".to_string(),
                        active_surface: active_surface,
                    }
                    if has_other_selected {
                        ArtifactSurfaceModeButton {
                            label: "Compare".to_string(),
                            icon: "compare".to_string(),
                            value: "compare".to_string(),
                            active_surface: active_surface,
                        }
                    }
                    ArtifactSurfaceModeButton {
                        label: "Handoff".to_string(),
                        icon: "forward_to_inbox".to_string(),
                        value: "handoff".to_string(),
                        active_surface: active_surface,
                    }
                }

                ArtifactTaskSurface {
                    mode: active_surface(),
                    artifact: artifact.clone(),
                    other_selected_labels: other_selected_labels.clone(),
                    starter_prompt: starter_prompt,
                }

                button {
                    class: "inline-flex items-center gap-2 rounded-xl border border-emerald-200 bg-emerald-50 px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-emerald-700 hover:bg-emerald-100 transition-all",
                    onclick: {
                        let db = db.clone();
                        let artifact = artifact.clone();
                        let selected_artifacts = selected_artifacts;
                        let mut selected_work_item = selected_work_item;
                        let mut work_items_resource = work_items_resource.clone();
                        move |_| {
                            let db = db.clone();
                            let artifact = artifact.clone();
                            let selected_artifacts = selected_artifacts;
                            let mut selected_work_item = selected_work_item;
                            let mut work_items_resource = work_items_resource.clone();
                            let active_mode = active_surface();
                            spawn(async move {
                                let (_, brief, prompt, _, _) =
                                    artifact_surface_definition(&active_mode, &artifact.kind);
                                let source_artifact_ids = selected_artifacts()
                                    .iter()
                                    .map(|item| item.id)
                                    .collect::<Vec<_>>();
                                let source_artifact_ids_json =
                                    serde_json::to_string(&source_artifact_ids).ok();
                                let title = format!(
                                    "{}: {}",
                                    prettify_token(&active_mode),
                                    artifact.label
                                );
                                if let Ok(id) = backend::create_work_item(
                                    &db,
                                    artifact.conversation_id,
                                    &title,
                                    Some(brief),
                                    "queued",
                                    &active_mode,
                                    artifact.domain.as_deref(),
                                    prompt,
                                    source_artifact_ids_json.as_deref(),
                                    None,
                                )
                                .await
                                {
                                    let now = chrono::Utc::now().naive_utc();
                                    selected_work_item.set(Some(backend::WorkItem {
                                        id,
                                        conversation_id: artifact.conversation_id,
                                        title,
                                        brief: Some(brief.to_string()),
                                        status: "queued".to_string(),
                                        surface_kind: active_mode,
                                        assigned_domain: artifact.domain.clone(),
                                        prompt: prompt.to_string(),
                                        source_artifact_ids_json,
                                        metadata_json: None,
                                        created_at: now,
                                        updated_at: now,
                                    }));
                                    work_items_resource.restart();
                                }
                            });
                        }
                    },
                    MaterialIcon { name: "add_task", size: 14 }
                    "Create Work Item"
                }
            }

            div { class: "space-y-2",
                p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Utilities" }
                div { class: "flex flex-wrap gap-2",
                    button {
                        class: format!(
                            "inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] transition-all {}",
                            if is_selected {
                                "border-blue-200 bg-blue-50 text-blue-700"
                            } else {
                                "border-slate-200 bg-white text-slate-700 hover:border-blue-200 hover:text-blue-700"
                            }
                        ),
                        onclick: {
                            let artifact = artifact.clone();
                            move |_| {
                                selected_artifacts.with_mut(|selected| {
                                    if let Some(index) = selected.iter().position(|item| item.id == artifact.id) {
                                        selected.remove(index);
                                    } else {
                                        selected.push(artifact.clone());
                                    }
                                });
                            }
                        },
                        MaterialIcon { name: if is_selected { "task_alt" } else { "add_circle" }, size: 14 }
                        if is_selected { "Selected for Prompt" } else { "Use in Next Prompt" }
                    }

                    button {
                        class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                        onclick: move |_| starter_prompt.set(Some("Summarize the selected artifact and tell me what matters most.".to_string())),
                        MaterialIcon { name: "summarize", size: 14 }
                        "Summarize"
                    }

                    if artifact.kind == "image" {
                        button {
                            class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                            onclick: move |_| starter_prompt.set(Some("Analyze the selected screenshot and describe what is visible, important, or unusual.".to_string())),
                            MaterialIcon { name: "image_search", size: 14 }
                            "Analyze Screenshot"
                        }
                    }

                    if artifact.kind == "file" {
                        button {
                            class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                            onclick: move |_| starter_prompt.set(Some("Read the selected file and summarize its contents.".to_string())),
                            MaterialIcon { name: "description", size: 14 }
                            "Read File"
                        }
                    }

                    if has_other_selected {
                        button {
                            class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                            onclick: move |_| starter_prompt.set(Some("Compare the selected artifacts and explain the important differences.".to_string())),
                            MaterialIcon { name: "compare", size: 14 }
                            "Compare Selected"
                        }
                    }

                    if let Some(path) = artifact.file_path.clone().filter(|value| !value.trim().is_empty()) {
                        button {
                            class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                            onclick: {
                                let path = path.clone();
                                move |_| on_open.call(path.clone())
                            },
                            MaterialIcon { name: "open_in_new", size: 14 }
                            "Open"
                        }
                        button {
                            class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                            onclick: {
                                let path = path.clone();
                                move |_| copy_to_clipboard(path.clone())
                            },
                            MaterialIcon { name: "content_copy", size: 14 }
                            "Copy Path"
                        }
                    } else if let Some(content) = artifact.content.clone().filter(|value| !value.trim().is_empty()) {
                        button {
                            class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                            onclick: {
                                let content = content.clone();
                                move |_| copy_to_clipboard(content.clone())
                            },
                            MaterialIcon { name: "content_copy", size: 14 }
                            "Copy Content"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ArtifactSurfaceModeButton(
    label: String,
    icon: String,
    value: String,
    active_surface: Signal<String>,
) -> Element {
    let is_active = active_surface() == value;

    rsx! {
        button {
            class: format!(
                "inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] transition-all {}",
                if is_active {
                    "border-blue-200 bg-blue-50 text-blue-700"
                } else {
                    "border-slate-200 bg-white text-slate-600 hover:border-blue-200 hover:text-blue-700"
                }
            ),
            onclick: move |_| active_surface.set(value.clone()),
            MaterialIcon { name: icon, size: 14 }
            "{label}"
        }
    }
}

#[component]
fn ArtifactTaskSurface(
    mode: String,
    artifact: backend::Artifact,
    other_selected_labels: Vec<String>,
    starter_prompt: Signal<Option<String>>,
) -> Element {
    let mode_key = mode.as_str();
    let (title, description, prompt, action_label, icon) =
        artifact_surface_definition(mode_key, &artifact.kind);

    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-white px-4 py-4 shadow-sm space-y-3",
            div {
                p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "{title}" }
                p { class: "text-xs text-slate-600 mt-2 leading-relaxed", "{description}" }
            }

            if mode_key == "compare" {
                if other_selected_labels.is_empty() {
                    div { class: "rounded-xl border border-dashed border-slate-200 bg-slate-50 px-3 py-3 text-xs text-slate-500",
                        "Select at least one more artifact to unlock a real compare surface."
                    }
                } else {
                    div { class: "space-y-2",
                        p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Also Selected" }
                        div { class: "flex flex-wrap gap-2",
                            for label in other_selected_labels {
                                span { class: "inline-flex items-center rounded-full border border-slate-200 bg-slate-50 px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.14em] text-slate-600",
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            } else if let Some(path) = artifact.file_path.clone().filter(|value| !value.trim().is_empty()) {
                div { class: "space-y-2",
                    p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Working From" }
                    p { class: "text-[11px] font-mono text-slate-500 break-all", "{path}" }
                }
            } else if let Some(content) = artifact.content.clone().filter(|value| !value.trim().is_empty()) {
                div { class: "space-y-2",
                    p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Working Excerpt" }
                    p { class: "text-xs text-slate-600 whitespace-pre-wrap break-words line-clamp-4", "{content}" }
                }
            }

            div { class: "rounded-xl border border-slate-200 bg-slate-50 px-3 py-3",
                p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Task Prompt" }
                p { class: "text-xs text-slate-700 mt-2 leading-relaxed", "{prompt}" }
            }

            button {
                class: "inline-flex items-center gap-2 rounded-xl border border-blue-200 bg-blue-50 px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-blue-700 hover:bg-blue-100 transition-all",
                onclick: {
                    let prompt = prompt.to_string();
                    move |_| starter_prompt.set(Some(prompt.clone()))
                },
                MaterialIcon { name: icon, size: 14 }
                "{action_label}"
            }
        }
    }
}

fn artifact_surface_definition<'a>(
    mode: &'a str,
    artifact_kind: &str,
) -> (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
) {
    match mode {
        "review" => (
            "Review Surface",
            "Use this when you want a grounded read on what matters, what looks wrong, or what deserves attention in the artifact.",
            match artifact_kind {
                "image" => "Review the selected screenshot carefully and call out important details, issues, or anomalies.",
                "file" => "Review the selected file and identify the most important details, risks, or follow-up questions.",
                _ => "Review the selected artifact and tell me what stands out, what matters, and what needs attention.",
            },
            "Start Review",
            "fact_check",
        ),
        "compare" => (
            "Compare Surface",
            "Turn the selected artifacts into a comparison task instead of relying on a generic follow-up in chat.",
            "Compare the selected artifacts and explain the important differences, shared patterns, and best next step.",
            "Compare Artifacts",
            "compare",
        ),
        "handoff" => (
            "Handoff Surface",
            "Package this artifact into a reusable handoff for the next agent, workflow, or human reviewer.",
            "Turn the selected artifact into a clean handoff with context, key details, and the recommended next action.",
            "Create Handoff",
            "forward_to_inbox",
        ),
        _ => (
            "Summary Surface",
            "Condense the artifact into the shortest useful understanding before you branch into deeper work.",
            "Summarize the selected artifact and tell me what matters most.",
            "Create Summary",
            "summarize",
        ),
    }
}

#[component]
fn WorkboardPanel(
    db: std::sync::Arc<crate::db::Database>,
    work_items: Vec<backend::WorkItem>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
) -> Element {
    rsx! {
        div { class: "border-l border-slate-200 border-b border-slate-200 bg-white/95 backdrop-blur-sm",
            div { class: "px-5 py-4 space-y-3",
                div {
                    p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Workboard" }
                    h3 { class: "text-sm font-black text-slate-900 mt-1", "Explicit work items for humans and agents to coordinate through." }
                }

                if work_items.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-slate-200 bg-slate-50 px-4 py-3 text-xs text-slate-500 leading-relaxed",
                        "No work items yet. Create one from an artifact surface to turn a prompt into a tracked unit of work."
                    }
                } else {
                    div { class: "space-y-2 max-h-72 overflow-y-auto custom-scrollbar pr-1",
                        for work_item in work_items {
                            WorkItemCard {
                                key: "{work_item.id}",
                                db: db.clone(),
                                work_item: work_item,
                                selected_work_item: selected_work_item,
                                starter_prompt: starter_prompt,
                                work_items_resource: work_items_resource.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn WorkItemCard(
    db: std::sync::Arc<crate::db::Database>,
    work_item: backend::WorkItem,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
) -> Element {
    let is_selected = selected_work_item()
        .as_ref()
        .map(|selected| selected.id == work_item.id)
        .unwrap_or(false);

    rsx! {
        div { class: format!(
                "rounded-2xl border px-4 py-3 space-y-3 transition-all {}",
                if is_selected {
                    "border-violet-200 bg-violet-50/60 shadow-sm"
                } else {
                    "border-slate-200 bg-slate-50/80"
                }
            ),
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0 space-y-1",
                    p { class: "text-xs font-black text-slate-900 truncate", "{work_item.title}" }
                    if let Some(brief) = work_item.brief.clone().filter(|value| !value.trim().is_empty()) {
                        p { class: "text-xs text-slate-600 line-clamp-2", "{brief}" }
                    }
                }
                WorkItemStatusBadge { status: work_item.status.clone() }
            }

            div { class: "flex flex-wrap items-center gap-2",
                span { class: "inline-flex items-center rounded-full border border-slate-200 bg-white px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-slate-500",
                    "{prettify_token(&work_item.surface_kind)}"
                }
                if let Some(domain) = work_item.assigned_domain.clone().filter(|value| !value.trim().is_empty()) {
                    span { class: "inline-flex items-center rounded-full border border-slate-200 bg-white px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-slate-500",
                        "{prettify_token(&domain)}"
                    }
                }
            }

            div { class: "flex flex-wrap gap-2",
                button {
                    class: format!(
                        "inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] transition-all {}",
                        if is_selected {
                            "border-violet-200 bg-white text-violet-700"
                        } else {
                            "border-slate-200 bg-white text-slate-700 hover:border-violet-200 hover:text-violet-700"
                        }
                    ),
                    onclick: {
                        let work_item = work_item.clone();
                        move |_| selected_work_item.set(Some(work_item.clone()))
                    },
                    MaterialIcon { name: "visibility", size: 14 }
                    if is_selected { "Focused" } else { "Focus" }
                }

                button {
                    class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                    onclick: {
                        let work_item = work_item.clone();
                        move |_| {
                            selected_work_item.set(Some(work_item.clone()));
                            starter_prompt.set(Some(work_item.prompt.clone()));
                        }
                    },
                    MaterialIcon { name: "play_arrow", size: 14 }
                    "Use in Composer"
                }

                for status in ["queued", "in_progress", "done"] {
                    button {
                        class: format!(
                            "inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] transition-all {}",
                            if work_item.status == status {
                                "border-emerald-200 bg-emerald-50 text-emerald-700"
                            } else {
                                "border-slate-200 bg-white text-slate-600 hover:border-emerald-200 hover:text-emerald-700"
                            }
                        ),
                        onclick: {
                            let db = db.clone();
                            let work_item_id = work_item.id;
                            let status = status.to_string();
                            let mut work_items_resource = work_items_resource.clone();
                            let mut selected_work_item = selected_work_item;
                            move |_| {
                                let db = db.clone();
                                let status = status.clone();
                                let mut work_items_resource = work_items_resource.clone();
                                let mut selected_work_item = selected_work_item;
                                spawn(async move {
                                    if backend::update_work_item_status(&db, work_item_id, &status).await.is_ok() {
                                        if let Some(mut current) = selected_work_item() {
                                            if current.id == work_item_id {
                                                current.status = status.clone();
                                                selected_work_item.set(Some(current));
                                            }
                                        }
                                        work_items_resource.restart();
                                    }
                                });
                            }
                        },
                        "{prettify_token(status)}"
                    }
                }
            }
        }
    }
}

#[component]
fn WorkItemStatusBadge(status: String) -> Element {
    let class = match status.as_str() {
        "done" | "completed" => "bg-emerald-50 text-emerald-700 border-emerald-200",
        "in_progress" | "running" => "bg-blue-50 text-blue-700 border-blue-200",
        _ => "bg-amber-50 text-amber-700 border-amber-200",
    };

    rsx! {
        span { class: format!("inline-flex items-center rounded-full border px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] {}", class),
            "{prettify_token(&status)}"
        }
    }
}

#[component]
fn ChatScreenChip(tone: String, label: String) -> Element {
    let class = match tone.as_str() {
        "blue" => "bg-blue-50 text-blue-700 border-blue-200",
        "violet" => "bg-violet-50 text-violet-700 border-violet-200",
        "green" => "bg-emerald-50 text-emerald-700 border-emerald-200",
        "red" => "bg-red-50 text-red-700 border-red-200",
        _ => "bg-slate-100 text-slate-600 border-slate-200",
    };

    rsx! {
        span { class: format!("inline-flex items-center rounded-full border px-3 py-1 text-[10px] font-black uppercase tracking-[0.18em] {}", class),
            "{label}"
        }
    }
}

#[component]
fn EmptyConversationState(on_prompt: EventHandler<String>) -> Element {
    let prompts = [
        (
            "Research something",
            "Find the top three AI agent reliability practices and summarize them with evidence.",
        ),
        (
            "Plan a workflow",
            "Create a workflow plan for a daily website monitoring task with traceability.",
        ),
        (
            "Use a tool",
            "Generate a simple bar chart from the values 4, 8, and 12 with labels A, B, and C.",
        ),
    ];

    rsx! {
        div { class: "rounded-3xl border border-slate-200 bg-white shadow-sm p-8 space-y-6",
            div { class: "space-y-3",
                p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Mission Control" }
                h2 { class: "text-2xl font-black text-slate-900", "Start a conversation and let Overseer route the work." }
                p { class: "text-sm text-slate-500 leading-relaxed max-w-2xl",
                    "Chat is the front door. The orchestrator decides whether your request should stay conversational, go to a single domain, or expand into a multi-step execution run with traceability."
                }
            }
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-3",
                for (label, prompt) in prompts {
                    button {
                        class: "text-left rounded-2xl border border-slate-200 bg-slate-50 hover:bg-white hover:border-blue-200 transition-all p-4 space-y-2",
                        onclick: {
                            let prompt = prompt.to_string();
                            move |_| on_prompt.call(prompt.clone())
                        },
                        p { class: "text-xs font-black uppercase tracking-[0.18em] text-slate-500", "{label}" }
                        p { class: "text-sm text-slate-700 leading-relaxed", "{prompt}" }
                    }
                }
            }
        }
    }
}
