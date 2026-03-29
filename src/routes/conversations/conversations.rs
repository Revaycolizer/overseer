// src/routes/conversations/conversations.rs
use super::chat_input::ChatInput;
use super::helpers::{build_query_with_context, copy_to_clipboard};
use super::message_bubble::{MessageBubble, ThinkingBubble};
use super::sidebar::Sidebar;
use crate::backend;
use crate::routes::execution_trace::ExecutionTracePanel;
use base64::{engine::general_purpose, Engine as _};
use dioxus::prelude::document::eval;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Default)]
struct WorkItemRuntimeMetadata {
    latest_run_id: Option<i64>,
    latest_run_status: Option<String>,
    latest_artifact_ids: Option<Vec<i64>>,
    latest_response_excerpt: Option<String>,
    last_executed_at: Option<String>,
}

fn parse_work_item_runtime_metadata(work_item: &backend::WorkItem) -> WorkItemRuntimeMetadata {
    work_item
        .metadata_json
        .as_deref()
        .and_then(|json| serde_json::from_str::<WorkItemRuntimeMetadata>(json).ok())
        .unwrap_or_default()
}

#[derive(Debug, Clone, PartialEq)]
enum WorkspaceSessionRef {
    Artifact(i64),
    WorkItem(i64),
}

fn workspace_session_key(session: &WorkspaceSessionRef) -> String {
    match session {
        WorkspaceSessionRef::Artifact(id) => format!("artifact:{id}"),
        WorkspaceSessionRef::WorkItem(id) => format!("work-item:{id}"),
    }
}

fn upsert_workspace_session(
    sessions: &mut Vec<WorkspaceSessionRef>,
    session: WorkspaceSessionRef,
) -> String {
    let key = workspace_session_key(&session);
    if !sessions
        .iter()
        .any(|existing| workspace_session_key(existing) == key)
    {
        sessions.push(session);
    }
    key
}

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
            let key = workspace_sessions.with_mut(|sessions| {
                upsert_workspace_session(sessions, WorkspaceSessionRef::Artifact(artifact.id))
            });
            active_workspace_session_key.set(Some(key));
        }
    });

    use_effect(move || {
        if let Some(work_item) = selected_work_item() {
            let key = workspace_sessions.with_mut(|sessions| {
                upsert_workspace_session(sessions, WorkspaceSessionRef::WorkItem(work_item.id))
            });
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
        } else if active_key.as_ref().is_none_or(|key| {
            !sessions
                .iter()
                .any(|session| workspace_session_key(session) == *key)
        }) {
            active_workspace_session_key.set(Some(workspace_session_key(&sessions[0])));
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
    let db_for_registry_status = db.clone();
    let runtime_registry_resource = use_resource(move || {
        let db = db_for_registry_status.clone();
        async move { backend::get_runtime_registry_status(&db).await }
    });
    let runtime_registry_status = runtime_registry_resource.read().cloned();
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
    let mut runtime_registry_poll = runtime_registry_resource.clone();
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            execution_runs_poll.restart();
            execution_events_poll.restart();
            artifacts_poll.restart();
            work_items_poll.restart();
            runtime_registry_poll.restart();
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
                            if !workspace_sessions().is_empty() {
                                div { class: "border-b border-slate-200 bg-white/70 px-6 py-4",
                                    div { class: "max-w-3xl mx-auto",
                                        WorkspaceSessionDeck {
                                            db: db.clone(),
                                            sessions: workspace_sessions(),
                                            sessions_signal: workspace_sessions,
                                            active_session_key: active_workspace_session_key,
                                            artifacts: artifacts.clone(),
                                            work_items: work_items.clone(),
                                            selected_artifacts: selected_artifacts,
                                            selected_work_item: selected_work_item,
                                            starter_prompt: starter_prompt,
                                            work_items_resource: work_items_resource.clone(),
                                            on_open: move |path: String| {
                                                spawn(async move {
                                                    let _ = backend::open_local_path(&path).await;
                                                });
                                            },
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
                            RuntimeRegistryPanel {
                                status: runtime_registry_status,
                            }
                            RecentArtifactsPanel {
                                artifacts: artifacts,
                                selected_artifacts: selected_artifacts,
                                focused_artifact: focused_artifact,
                                on_open: move |path: String| {
                                    spawn(async move {
                                        let _ = backend::open_local_path(&path).await;
                                    });
                                },
                            }
                            WorkboardPanel {
                                db: db.clone(),
                                messages_resource: messages_resource.clone(),
                                visible_messages: visible_messages,
                                work_items: work_items,
                                selected_work_item: selected_work_item,
                                is_thinking: is_thinking,
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

#[component]
fn RuntimeRegistryPanel(status: Option<backend::RuntimeRegistryStatus>) -> Element {
    let status = status.unwrap_or(backend::RuntimeRegistryStatus {
        status: "unknown".to_string(),
        refreshed_at: None,
        error: None,
    });

    let tone_class = match status.status.as_str() {
        "live" => "border-emerald-200 bg-emerald-50 text-emerald-700",
        "error" => "border-red-200 bg-red-50 text-red-700",
        _ => "border-slate-200 bg-slate-50 text-slate-600",
    };

    rsx! {
        div { class: "border-b border-slate-200 bg-white p-6 space-y-3",
            p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Runtime Registry" }
            div { class: "flex items-center justify-between gap-3",
                h3 { class: "text-sm font-black text-slate-900", "Live runtime sync" }
                span { class: "rounded-full border px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] {tone_class}",
                    "{status.status}"
                }
            }
            p { class: "text-xs text-slate-500 leading-relaxed",
                "Shows whether the staged agent/tool/supervisor registry is currently live in the runtime."
            }
            if let Some(refreshed_at) = status.refreshed_at {
                p { class: "text-xs text-slate-600", "Last refreshed: {refreshed_at}" }
            } else {
                p { class: "text-xs text-slate-500", "No refresh timestamp recorded yet." }
            }
            if let Some(error) = status.error.filter(|value| !value.trim().is_empty()) {
                p { class: "text-xs text-red-700 whitespace-pre-wrap break-words", "{error}" }
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
fn WorkspaceSessionDeck(
    db: std::sync::Arc<crate::db::Database>,
    sessions: Vec<WorkspaceSessionRef>,
    sessions_signal: Signal<Vec<WorkspaceSessionRef>>,
    active_session_key: Signal<Option<String>>,
    artifacts: Vec<backend::Artifact>,
    work_items: Vec<backend::WorkItem>,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
    on_open: EventHandler<String>,
) -> Element {
    let active_key = active_session_key().or_else(|| sessions.first().map(workspace_session_key));
    let active_session = active_key.as_ref().and_then(|key| {
        sessions
            .iter()
            .find(|session| workspace_session_key(session) == *key)
            .cloned()
    });

    rsx! {
        div { class: "space-y-4",
            div { class: "flex items-center justify-between gap-3",
                div {
                    p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "Workspace Sessions" }
                    p { class: "text-xs text-slate-500 mt-1", "Keep artifact and work-item contexts open as switchable task surfaces." }
                }
                span { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "{sessions.len()} open" }
            }

            div { class: "flex flex-wrap gap-2",
                for session in sessions.iter().cloned() {
                    WorkspaceSessionTab {
                        session: session,
                        sessions_signal: sessions_signal,
                        active_session_key: active_session_key,
                        active_key: active_key.clone(),
                        sessions: sessions.clone(),
                        artifacts: artifacts.clone(),
                        work_items: work_items.clone(),
                    }
                }
            }

            if let Some(session) = active_session {
                match session {
                    WorkspaceSessionRef::Artifact(artifact_id) => {
                        if let Some(artifact) = artifacts.iter().find(|artifact| artifact.id == artifact_id).cloned() {
                            rsx! {
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
                        } else {
                            rsx! {
                                div { class: "rounded-2xl border border-dashed border-slate-200 bg-white px-4 py-5 text-sm text-slate-500",
                                    "This artifact is no longer available in the current conversation state."
                                }
                            }
                        }
                    }
                    WorkspaceSessionRef::WorkItem(work_item_id) => {
                        if let Some(work_item) = work_items.iter().find(|work_item| work_item.id == work_item_id).cloned() {
                            rsx! {
                                WorkItemWorkspacePanel {
                                    work_item: work_item,
                                    selected_work_item: selected_work_item,
                                    starter_prompt: starter_prompt,
                                }
                            }
                        } else {
                            rsx! {
                                div { class: "rounded-2xl border border-dashed border-slate-200 bg-white px-4 py-5 text-sm text-slate-500",
                                    "This work item is no longer available in the current conversation state."
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn WorkspaceSessionTab(
    session: WorkspaceSessionRef,
    sessions_signal: Signal<Vec<WorkspaceSessionRef>>,
    active_session_key: Signal<Option<String>>,
    active_key: Option<String>,
    sessions: Vec<WorkspaceSessionRef>,
    artifacts: Vec<backend::Artifact>,
    work_items: Vec<backend::WorkItem>,
) -> Element {
    let session_key = workspace_session_key(&session);
    let is_active = active_key.as_ref().is_some_and(|key| key == &session_key);
    let label = match session {
        WorkspaceSessionRef::Artifact(artifact_id) => artifacts
            .iter()
            .find(|artifact| artifact.id == artifact_id)
            .map(|artifact| artifact.label.clone())
            .unwrap_or_else(|| format!("Artifact {artifact_id}")),
        WorkspaceSessionRef::WorkItem(work_item_id) => work_items
            .iter()
            .find(|work_item| work_item.id == work_item_id)
            .map(|work_item| work_item.title.clone())
            .unwrap_or_else(|| format!("Work Item {work_item_id}")),
    };

    rsx! {
        div { class: format!(
                "inline-flex items-center gap-2 rounded-2xl border px-3 py-2 transition-all {}",
                if is_active {
                    "border-blue-200 bg-blue-50 text-blue-700"
                } else {
                    "border-slate-200 bg-white text-slate-700"
                }
            ),
            button {
                class: "text-[10px] font-black uppercase tracking-[0.14em]",
                onclick: {
                    let session_key = session_key.clone();
                    move |_| active_session_key.set(Some(session_key.clone()))
                },
                "{label}"
            }
            button {
                class: "inline-flex size-5 items-center justify-center rounded-full text-slate-400 hover:bg-slate-100 hover:text-slate-700 transition-all",
                onclick: {
                    let session_key = session_key.clone();
                    let sessions = sessions.clone();
                    move |_| {
                        let next_key = sessions
                            .iter()
                            .filter(|existing| workspace_session_key(existing) != session_key)
                            .map(workspace_session_key)
                            .next();
                        sessions_signal.with_mut(|open_sessions| {
                            open_sessions.retain(|existing| workspace_session_key(existing) != session_key);
                        });
                        active_session_key.with_mut(|active| {
                            if active.as_ref().is_some_and(|current| current == &session_key) {
                                *active = next_key;
                            }
                        });
                    }
                },
                MaterialIcon { name: "close", size: 14 }
            }
        }
    }
}

#[component]
fn WorkItemWorkspacePanel(
    work_item: backend::WorkItem,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    starter_prompt: Signal<Option<String>>,
) -> Element {
    let runtime_metadata = parse_work_item_runtime_metadata(&work_item);

    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-gradient-to-br from-slate-50 to-white px-4 py-4 shadow-sm space-y-4",
            div {
                p { class: "text-[10px] font-black uppercase tracking-[0.2em] text-slate-400", "Work Item Session" }
                h4 { class: "text-sm font-black text-slate-900 mt-1", "{work_item.title}" }
                if let Some(brief) = work_item.brief.clone().filter(|value| !value.trim().is_empty()) {
                    p { class: "text-xs text-slate-500 mt-1 leading-relaxed", "{brief}" }
                }
            }

            div { class: "flex flex-wrap items-center gap-2",
                WorkItemStatusBadge { status: work_item.status.clone() }
                span { class: "inline-flex items-center rounded-full border border-slate-200 bg-white px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-slate-500",
                    "{prettify_token(&work_item.surface_kind)}"
                }
                if let Some(domain) = work_item.assigned_domain.clone().filter(|value| !value.trim().is_empty()) {
                    span { class: "inline-flex items-center rounded-full border border-slate-200 bg-white px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-slate-500",
                        "{prettify_token(&domain)}"
                    }
                }
            }

            div { class: "rounded-xl border border-slate-200 bg-white px-4 py-3 space-y-2",
                p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Prompt" }
                p { class: "text-sm text-slate-700 whitespace-pre-wrap break-words", "{work_item.prompt}" }
            }

            if let Some(excerpt) = runtime_metadata.latest_response_excerpt.filter(|value| !value.trim().is_empty()) {
                div { class: "rounded-xl border border-slate-200 bg-white px-4 py-3 space-y-2",
                    div { class: "flex flex-wrap items-center justify-between gap-2",
                        p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Latest Result" }
                        if let Some(status) = runtime_metadata.latest_run_status.filter(|value| !value.trim().is_empty()) {
                            span { class: "text-[10px] font-black uppercase tracking-[0.16em] text-slate-500",
                                "{prettify_token(&status)}"
                            }
                        }
                    }
                    p { class: "text-sm text-slate-700 whitespace-pre-wrap break-words", "{excerpt}" }
                }
            }

            div { class: "flex flex-wrap gap-2",
                button {
                    class: "inline-flex items-center gap-2 rounded-xl border border-violet-200 bg-violet-50 px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-violet-700 hover:bg-violet-100 transition-all",
                    onclick: {
                        let work_item = work_item.clone();
                        move |_| {
                            selected_work_item.set(Some(work_item.clone()));
                            starter_prompt.set(Some(work_item.prompt.clone()));
                        }
                    },
                    MaterialIcon { name: "edit_note", size: 14 }
                    "Use in Composer"
                }
                button {
                    class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:text-blue-700 transition-all",
                    onclick: {
                        let prompt = work_item.prompt.clone();
                        move |_| copy_to_clipboard(prompt.clone())
                    },
                    MaterialIcon { name: "content_copy", size: 14 }
                    "Copy Prompt"
                }
            }
        }
    }
}

#[component]
fn RecentArtifactsPanel(
    artifacts: Vec<backend::Artifact>,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    focused_artifact: Signal<Option<backend::Artifact>>,
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
                    "Session Open"
                } else {
                    "Open Session"
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
    messages_resource: Resource<Vec<backend::Message>>,
    visible_messages: Signal<Vec<backend::Message>>,
    work_items: Vec<backend::WorkItem>,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    is_thinking: Signal<bool>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
) -> Element {
    let global_state = crate::state::use_global_state();
    let agent_system = global_state.read().agent_system.clone();

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
                                agent_system: agent_system.clone(),
                                messages_resource: messages_resource.clone(),
                                visible_messages: visible_messages,
                                work_item: work_item,
                                selected_work_item: selected_work_item,
                                is_thinking: is_thinking,
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
    agent_system: std::sync::Arc<crate::engine::system::AgentSystem>,
    messages_resource: Resource<Vec<backend::Message>>,
    visible_messages: Signal<Vec<backend::Message>>,
    work_item: backend::WorkItem,
    selected_work_item: Signal<Option<backend::WorkItem>>,
    is_thinking: Signal<bool>,
    starter_prompt: Signal<Option<String>>,
    work_items_resource: Resource<Vec<backend::WorkItem>>,
) -> Element {
    let is_selected = selected_work_item()
        .as_ref()
        .map(|selected| selected.id == work_item.id)
        .unwrap_or(false);
    let runtime_metadata = parse_work_item_runtime_metadata(&work_item);
    let latest_artifact_count = runtime_metadata
        .latest_artifact_ids
        .as_ref()
        .map(|artifacts| artifacts.len())
        .unwrap_or(0);

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

            if runtime_metadata.latest_run_id.is_some() {
                div { class: "rounded-xl border border-slate-200 bg-white/90 px-3 py-2 space-y-1",
                    div { class: "flex items-center justify-between gap-2",
                        p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Latest Run" }
                        if let Some(status) = runtime_metadata.latest_run_status.clone().filter(|value| !value.trim().is_empty()) {
                            span { class: "text-[10px] font-black uppercase tracking-[0.16em] text-slate-500",
                                "{prettify_token(&status)}"
                            }
                        }
                    }
                    if let Some(excerpt) = runtime_metadata
                        .latest_response_excerpt
                        .clone()
                        .filter(|value| !value.trim().is_empty())
                    {
                        p { class: "text-xs text-slate-600 line-clamp-3", "{excerpt}" }
                    }
                    div { class: "flex flex-wrap items-center gap-2 text-[10px] font-semibold text-slate-400",
                        if let Some(run_id) = runtime_metadata.latest_run_id {
                            span { "Run #{run_id}" }
                        }
                        if latest_artifact_count > 0 {
                            span { "{latest_artifact_count} artifact(s)" }
                        }
                        if let Some(last_executed_at) = runtime_metadata
                            .last_executed_at
                            .clone()
                            .filter(|value| !value.trim().is_empty())
                        {
                            span { "{last_executed_at}" }
                        }
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
                    if is_selected { "Session Open" } else { "Open Session" }
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

                button {
                    class: format!(
                        "inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] transition-all {}",
                        if is_thinking() {
                            "border-slate-200 bg-slate-100 text-slate-400 cursor-not-allowed"
                        } else {
                            "border-blue-200 bg-blue-50 text-blue-700 hover:bg-blue-100"
                        }
                    ),
                    disabled: is_thinking(),
                    onclick: {
                        let db = db.clone();
                        let agent_system = agent_system.clone();
                        let work_item = work_item.clone();
                        let mut messages_resource = messages_resource.clone();
                        let mut visible_messages = visible_messages;
                        let mut selected_work_item = selected_work_item;
                        let mut is_thinking = is_thinking;
                        let mut work_items_resource = work_items_resource.clone();
                        move |_| {
                            let db = db.clone();
                            let agent_system = agent_system.clone();
                            let work_item = work_item.clone();
                            let mut messages_resource = messages_resource.clone();
                            let mut visible_messages = visible_messages;
                            let mut selected_work_item = selected_work_item;
                            let mut is_thinking = is_thinking;
                            let mut work_items_resource = work_items_resource.clone();
                            let optimistic_id = -chrono::Utc::now().timestamp_micros();
                            visible_messages.with_mut(|messages| {
                                messages.push(backend::Message {
                                    id: optimistic_id,
                                    conversation_id: work_item.conversation_id,
                                    role: "user".to_string(),
                                    content: work_item.prompt.clone(),
                                    created_at: chrono::Utc::now().naive_utc(),
                                    attachments: vec![],
                                    hide_evidence: false,
                                });
                            });
                            spawn(async move {
                                selected_work_item.set(Some(work_item.clone()));
                                let _ = backend::update_work_item_status(&db, work_item.id, "in_progress").await;
                                work_items_resource.restart();
                                let source_artifact_ids = work_item
                                    .source_artifact_ids_json
                                    .as_deref()
                                    .and_then(|json| serde_json::from_str::<Vec<i64>>(json).ok())
                                    .unwrap_or_default();
                                let source_artifacts = backend::get_artifacts_by_ids(&db, &source_artifact_ids)
                                    .await
                                    .unwrap_or_default();
                                let query = build_query_with_context(
                                    &work_item.prompt,
                                    &source_artifacts,
                                    Some(&work_item),
                                );

                                let add_user = backend::add_message(
                                    &db,
                                    work_item.conversation_id,
                                    "user",
                                    &query,
                                    false,
                                )
                                .await;

                                if add_user.is_err() {
                                    visible_messages.write().retain(|message| message.id != optimistic_id);
                                    let _ = backend::update_work_item_status(&db, work_item.id, "blocked").await;
                                    work_items_resource.restart();
                                    return;
                                }

                                messages_resource.restart();
                                is_thinking.set(true);
                                let response = agent_system
                                    .orchestrate(&db, work_item.conversation_id, &query)
                                    .await;
                                is_thinking.set(false);

                                let _ = backend::add_message(
                                    &db,
                                    work_item.conversation_id,
                                    "assistant",
                                    &response,
                                    false,
                                )
                                .await;
                                messages_resource.restart();

                                let latest_run = backend::get_latest_execution_run_for_conversation(
                                    &db,
                                    work_item.conversation_id,
                                )
                                .await
                                .ok()
                                .flatten();
                                let (status, metadata_json) = if let Some(run) = latest_run {
                                    let latest_artifacts = backend::get_artifacts_for_execution_run(&db, run.id)
                                        .await
                                        .unwrap_or_default();
                                    let work_status = if run.status == "completed" {
                                        "done"
                                    } else {
                                        "blocked"
                                    };
                                    let excerpt = run
                                        .final_response
                                        .clone()
                                        .filter(|value: &String| !value.trim().is_empty())
                                        .unwrap_or_else(|| response.clone());
                                    let metadata = serde_json::json!({
                                        "latest_run_id": run.id,
                                        "latest_run_status": run.status,
                                        "latest_artifact_ids": latest_artifacts.iter().map(|artifact| artifact.id).collect::<Vec<_>>(),
                                        "latest_response_excerpt": excerpt.chars().take(220).collect::<String>(),
                                        "last_executed_at": run
                                            .finished_at
                                            .map(|finished: chrono::NaiveDateTime| finished.to_string())
                                            .unwrap_or_else(|| run.started_at.to_string()),
                                    });
                                    (work_status, Some(metadata.to_string()))
                                } else {
                                    ("blocked", None)
                                };
                                let _ = backend::update_work_item_runtime(
                                    &db,
                                    work_item.id,
                                    status,
                                    metadata_json.as_deref(),
                                )
                                .await;
                                if let Some(mut current) = selected_work_item() {
                                    if current.id == work_item.id {
                                        current.status = status.to_string();
                                        current.metadata_json = metadata_json.clone();
                                        selected_work_item.set(Some(current));
                                    }
                                }
                                work_items_resource.restart();
                            });
                        }
                    },
                    MaterialIcon { name: if is_thinking() { "hourglass_top" } else { "smart_toy" }, size: 14 }
                    if is_thinking() { "Running" } else { "Run Now" }
                }

                for status in ["queued", "in_progress", "done", "blocked"] {
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
