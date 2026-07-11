use crate::backend;
use crate::routes::conversations::helpers::build_query_with_context;
use crate::state::use_global_state;
use base64::{engine::general_purpose, Engine as _};
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[derive(Props, Clone, PartialEq)]
pub struct ChatInputProps {
    pub selected_conversation_id: Signal<Option<i64>>,
    pub messages_resource: Resource<Vec<backend::Message>>,
    pub visible_messages: Signal<Vec<backend::Message>>,
    pub conversations_resource: Resource<Vec<backend::Conversation>>,
    pub selected_artifacts: Signal<Vec<backend::Artifact>>,
    pub selected_work_item: Signal<Option<backend::WorkItem>>,
    pub is_first_msg: bool,
    pub is_thinking: Signal<bool>,
    pub starter_prompt: Signal<Option<String>>,
}

#[component]
pub fn ChatInput(props: ChatInputProps) -> Element {
    let global_state = use_global_state();
    let global_read = global_state.read();
    let db = global_read.db.clone();
    drop(global_read);

    let selected_artifacts = props.selected_artifacts;
    let mut selected_work_item = props.selected_work_item;
    let selected_conversation_id = props.selected_conversation_id;
    let starter_prompt = props.starter_prompt;
    let selected_artifact_count = selected_artifacts().len();
    let has_active_work_item = selected_work_item().is_some();
    let db_for_computer_state = db.clone();

    let computer_state_resource = use_resource(move || {
        let db = db_for_computer_state.clone();
        async move { backend::get_computer_supervision_state(&db).await }
    });

    let computer_state = computer_state_resource.read().as_ref().and_then(|r| r.as_ref().ok().cloned());
    let supervision_mode = computer_state
        .as_ref()
        .map(|state| state.supervision_mode.clone())
        .unwrap_or_else(|| "approval_required".to_string());
    let session_state = computer_state
        .as_ref()
        .map(|state| state.session_state.clone())
        .unwrap_or_else(|| "idle".to_string());
    let approval_state = computer_state
        .as_ref()
        .map(|state| state.approval_state.clone())
        .unwrap_or_else(|| "idle".to_string());
    let pending_action = computer_state
        .as_ref()
        .map(|state| state.pending_action.clone())
        .unwrap_or_default();
    let session_label = match session_state.as_str() {
        "approval_session" => "Approval session",
        "observing" => "Observe mode",
        "trusted_session" => "Trusted session",
        "paused" => "Paused",
        "emergency_stopped" => "Emergency stop",
        _ => "No active session",
    };

    rsx! {
        div { class: "p-6 bg-gradient-to-t from-[var(--workspace-bg)] via-[var(--workspace-bg)]/95 to-transparent",
            div { class: "max-w-3xl mx-auto",
                if selected_artifact_count > 0 {
                    ArtifactWorkspaceSurface {
                        selected_artifacts: selected_artifacts,
                        starter_prompt: starter_prompt,
                    }
                }
                if let Some(work_item) = selected_work_item() {
                    div { class: "mb-3 flex items-start justify-between gap-3 rounded-2xl border border-violet-200 bg-violet-50/80 px-4 py-3 shadow-sm",
                        div { class: "min-w-0 space-y-1",
                            p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-violet-600", "Active Work Item" }
                            p { class: "text-sm font-black text-slate-900 break-words", "{work_item.title}" }
                            p { class: "text-xs text-slate-600 line-clamp-2", "{work_item.prompt}" }
                        }
                        button {
                            class: "rounded-xl border border-violet-200 bg-white p-2 text-violet-400 hover:bg-violet-100 hover:text-violet-700 transition-all",
                            onclick: move |_| selected_work_item.set(None),
                            MaterialIcon { name: "close", size: 16 }
                        }
                    }
                }

                ChatComposer {
                    selected_conversation_id: selected_conversation_id,
                    messages_resource: props.messages_resource,
                    visible_messages: props.visible_messages,
                    conversations_resource: props.conversations_resource,
                    selected_artifacts: selected_artifacts,
                    selected_work_item: selected_work_item,
                    is_thinking: props.is_thinking,
                    starter_prompt: props.starter_prompt,
                    is_first_msg: props.is_first_msg,
                }

                div { class: "mt-3 flex flex-wrap items-center gap-2 px-1",
                    div { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[11px] text-slate-600 shadow-sm",
                        MaterialIcon { name: "policy", size: 14 }
                        span { class: "font-bold text-slate-500", "Mode" }
                        select {
                            class: "border-0 bg-transparent pr-5 text-[11px] font-semibold text-slate-700 outline-none",
                            value: "{supervision_mode}",
                            onchange: {
                                let db = db.clone();
                                let computer_state_resource = computer_state_resource.clone();
                                move |evt| {
                                    let db = db.clone();
                                    let mut computer_state_resource = computer_state_resource.clone();
                                    let value = evt.value();
                                    spawn(async move {
                                        let _ = backend::set_setting(
                                            &db,
                                            "computer_supervision_mode",
                                            &value,
                                        )
                                        .await;
                                        computer_state_resource.restart();
                                    });
                                }
                            },
                            option { value: "approval_required", "Approval Required" }
                            option { value: "observe_only", "Observe Only" }
                            option { value: "session_trusted", "Trusted Session" }
                            option { value: "trusted_auto_run", "Trusted Auto-Run" }
                        }
                    }
                    div { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-slate-50 px-3 py-2 text-[11px] text-slate-600 shadow-sm",
                        MaterialIcon { name: "verified_user", size: 14 }
                        span { class: "font-semibold text-slate-700", "{session_label}" }
                        if approval_state == "pending" {
                            span { class: "rounded-full bg-amber-100 px-2 py-0.5 text-[10px] font-black uppercase tracking-[0.14em] text-amber-700",
                                "Pending approval"
                            }
                        }
                    }
                    if approval_state == "pending" && !pending_action.trim().is_empty() {
                        div { class: "flex items-center gap-2 min-w-full rounded-2xl border border-amber-200 bg-amber-50/80 px-3 py-3 text-[11px] text-amber-900 shadow-sm",
                            MaterialIcon { name: "fact_check", size: 16, color: MaterialIconColor::Custom("#b45309".into()) }
                            p { class: "flex-1 leading-relaxed break-words", "{pending_action}" }
                            button {
                                class: "inline-flex items-center gap-2 rounded-xl bg-emerald-600 px-3 py-2 font-bold text-white hover:bg-emerald-700 transition-all",
                                onclick: {
                                    let db = db.clone();
                                    let computer_state_resource = computer_state_resource.clone();
                                    move |_| {
                                        let db = db.clone();
                                        let mut computer_state_resource = computer_state_resource.clone();
                                        spawn(async move {
                                            let _ = backend::approve_pending_computer_action(&db).await;
                                            computer_state_resource.restart();
                                        });
                                    }
                                },
                                MaterialIcon { name: "task_alt", size: 14, color: MaterialIconColor::Light }
                                "Approve"
                            }
                            button {
                                class: "inline-flex items-center gap-2 rounded-xl border border-red-200 bg-white px-3 py-2 font-bold text-red-600 hover:bg-red-50 transition-all",
                                onclick: {
                                    let db = db.clone();
                                    let computer_state_resource = computer_state_resource.clone();
                                    move |_| {
                                        let db = db.clone();
                                        let mut computer_state_resource = computer_state_resource.clone();
                                        spawn(async move {
                                            let _ = backend::reject_pending_computer_action(&db).await;
                                            computer_state_resource.restart();
                                        });
                                    }
                                },
                                MaterialIcon { name: "cancel", size: 14, color: MaterialIconColor::Custom("#dc2626".into()) }
                                "Reject"
                            }
                        }
                    }
                }
                div { class: "mt-3 flex items-center justify-between gap-3 px-1 text-[11px] text-slate-500",
                    div { class: "flex items-center gap-2 min-w-0",
                        MaterialIcon { name: "radar", size: 14 }
                        span {
                            if has_active_work_item {
                                "Your next message will include the active work item as structured coordination context."
                            } else if selected_artifact_count > 0 {
                                "Your next message will include {selected_artifact_count} selected artifact(s) as structured context."
                            } else if matches!(supervision_mode.as_str(), "approval_required" | "observe_only" | "session_trusted" | "trusted_auto_run") {
                                "Desktop sessions will start automatically when a computer task needs them."
                            } else if (props.is_thinking)() {
                                "Overseer is processing the current request."
                            } else if selected_conversation_id().is_some() {
                                "Conversation is active. Trace events will appear automatically."
                            } else {
                                "A new conversation will be created automatically when you send."
                            }
                        }
                    }
                    span { class: "shrink-0", "Enter to send • Shift+Enter for newline" }
                }
            }
        }
    }
}

#[component]
fn ChatComposer(props: ChatInputProps) -> Element {
    let global_state = use_global_state();
    let global_read = global_state.read();
    let db = global_read.db.clone();
    let agent_system = global_read.agent_system.clone();
    drop(global_read);

    let db_for_send = db.clone();
    let db_for_stop = db.clone();

    let mut input_text = use_signal(|| String::new());
    let mut pending_attachments = use_signal(|| Vec::<(String, String, Vec<u8>)>::new());
    let is_thinking = props.is_thinking;
    let mut active_request_abort_handle = use_signal(|| None::<tokio::task::AbortHandle>);
    let mut is_local_executing = use_context::<Signal<bool>>();
    let execution_runs_resource = use_context::<Resource<Vec<backend::ExecutionRun>>>();

    let messages_resource = props.messages_resource;
    let mut visible_messages = props.visible_messages;
    let conversations_resource = props.conversations_resource;
    let mut selected_artifacts = props.selected_artifacts;
    let selected_work_item = props.selected_work_item;
    let selected_conversation_id = props.selected_conversation_id;
    let is_first_msg = props.is_first_msg;
    let mut starter_prompt = props.starter_prompt;

    use_effect(move || {
        if let Some(prompt) = starter_prompt() {
            input_text.set(prompt);
            starter_prompt.set(None);
        }
    });

    let send = use_callback(move |()| {
        let val = input_text();
        if val.trim().is_empty() || is_thinking() {
            return;
        }
        is_local_executing.set(true);
        input_text.set(String::new());
        let attachments = pending_attachments();
        pending_attachments.set(vec![]);
        let artifacts_for_prompt = selected_artifacts();
        let work_item_for_prompt = selected_work_item();
        selected_artifacts.set(Vec::new());
        let query_with_artifacts =
            build_query_with_context(&val, &artifacts_for_prompt, work_item_for_prompt.as_ref());

        let db_inner = db_for_send.clone();
        let agent_inner = agent_system.clone();
        let mut messages_inner = messages_resource.clone();
        let mut visible_messages_inner = visible_messages;
        let mut conversations_inner = conversations_resource.clone();
        let conv_id = selected_conversation_id();
        let mut selected_conversation_id_inner = selected_conversation_id.clone();
        let mut active_request_abort_handle_inner = active_request_abort_handle;
        let mut is_local_executing_inner = is_local_executing;
        let mut execution_runs_resource_inner = execution_runs_resource;
        let query = val.clone();
        let data_dir = crate::db::get_overseer_data_dir();
        let overseer_files_dir = data_dir.join("overseer_files");
        let optimistic_id = -chrono::Utc::now().timestamp_micros();
        let optimistic_attachments: Vec<backend::Attachment> = attachments.iter().map(|(name, mime, data)| {
            backend::Attachment {
                id: 0, message_id: 0, filename: name.clone(),
                file_path: overseer_files_dir.join(name).to_string_lossy().to_string(),
                content_type: mime.clone(), size: data.len() as i64, extracted_content: None,
            }
        }).collect();

        if let Some(id) = conv_id {
            visible_messages.with_mut(|messages| {
                messages.push(backend::Message {
                    id: optimistic_id,
                    conversation_id: id,
                    role: "user".to_string(),
                    content: val.clone(),
                    created_at: chrono::Utc::now().naive_utc(),
                    attachments: optimistic_attachments.clone(),
                    hide_evidence: false,
                });
            });
        }

        let is_first_msg_val = is_first_msg;

        let _task = spawn(async move {
            let mut resolved_conv_id = conv_id;
            let mut created_new_conversation = false;

            if resolved_conv_id.is_none() {
                match backend::add_conversation(&db_inner, "New Conversation").await {
                    Ok(id) => {
                        created_new_conversation = true;
                        resolved_conv_id = Some(id);
                        selected_conversation_id_inner.set(Some(id));
                        conversations_inner.restart();
                        visible_messages_inner.set(vec![backend::Message {
                            id: optimistic_id,
                            conversation_id: id,
                            role: "user".to_string(),
                            content: query.clone(),
                            created_at: chrono::Utc::now().naive_utc(),
                            attachments: optimistic_attachments.clone(),
                            hide_evidence: false,
                        }]);
                    }
                    Err(_) => {
                        is_local_executing_inner.set(false);
                        active_request_abort_handle_inner.set(None);
                        return;
                    }
                }
            }
            let conversation_id = resolved_conv_id.unwrap();

            let optimistic_title = if is_first_msg_val || created_new_conversation {
                let t = query.split_whitespace().take(4).collect::<Vec<_>>().join(" ");
                let _ = backend::update_conversation_title(&db_inner, conversation_id, &t).await;
                conversations_inner.restart();
                Some(t)
            } else {
                None
            };

            let (title_tx, title_rx) = tokio::sync::oneshot::channel::<()>();
            let mut conversations_for_title = conversations_inner.clone();
            spawn(async move {
                if title_rx.await.is_ok() {
                    conversations_for_title.restart();
                }
            });

            let handle = tokio::spawn(async move {
                if let Ok(msg_id) = backend::add_message(
                    &db_inner,
                    conversation_id,
                    "user",
                    &query_with_artifacts,
                    false,
                )
                .await
                {
                    if let Some(_optimistic) = optimistic_title.clone() {
                        let db_for_title = db_inner.clone();
                        let agent_for_title = agent_inner.clone();
                        let query_for_title = query.clone();
                        let conv_id_for_title = conversation_id;
                        let title_tx = title_tx;
                        tokio::spawn(async move {
                            let real_title = agent_for_title.generate_title(&db_for_title, &query_for_title).await;
                            if !real_title.is_empty() {
                                let _ = backend::update_conversation_title(&db_for_title, conv_id_for_title, &real_title).await;
                                let _ = title_tx.send(());
                            }
                        });
                    }
                    for (name, mime, data) in attachments {
                        let folder = &overseer_files_dir;
                        let _ = std::fs::create_dir_all(folder);
                        let path = folder.join(&name);
                        let path_str = path.to_string_lossy().to_string();
                        let _ = std::fs::write(&path, &data);
                        let _ = backend::add_attachment(
                            &db_inner,
                            msg_id,
                            &name,
                            &path_str,
                            &mime,
                            data.len() as i64,
                        )
                        .await;
                    }

                    let response = agent_inner
                        .orchestrate(&db_inner, conversation_id, &query_with_artifacts)
                        .await;

                    let _ =
                        backend::add_message(&db_inner, conversation_id, "assistant", &response, false)
                            .await;
                    true
                } else {
                    false
                }
            });

            active_request_abort_handle_inner.set(Some(handle.abort_handle()));

            let success = match handle.await {
                Ok(res) => res,
                Err(_) => false,
            };

            if success {
                messages_inner.restart();
                conversations_inner.restart();
                execution_runs_resource_inner.restart();
                is_local_executing_inner.set(false);
                active_request_abort_handle_inner.set(None);
            } else {
                is_local_executing_inner.set(false);
                visible_messages_inner
                    .write()
                    .retain(|message| message.id != optimistic_id);
                active_request_abort_handle_inner.set(None);
            }
        });
    });

    let stop = use_callback(move |()| {
        if let Some(abort_handle) = active_request_abort_handle.take() {
            abort_handle.abort();
        }
        is_local_executing.set(false);

        if let Some(conversation_id) = selected_conversation_id() {
            let db = db_for_stop.clone();
            spawn(async move {
                let _ = backend::cancel_latest_execution_run_for_conversation(&db, conversation_id)
                    .await;
            });
        }
    });

    rsx! {
        if !pending_attachments().is_empty() {
            div { class: "mb-3 flex flex-wrap gap-2",
                for (i, (name, _, _)) in pending_attachments().iter().enumerate() {
                    div { class: "flex items-center gap-2 rounded-xl border border-blue-100 bg-blue-50/80 px-3 py-2 text-[10px] font-black tracking-[0.08em] text-blue-700 shadow-sm",
                        MaterialIcon { name: "insert_drive_file", size: 16 }
                        "{name}"
                        button {
                            class: "ml-1 rounded-md p-0.5 text-blue-400 hover:bg-blue-100 hover:text-blue-700 transition-colors",
                            onclick: move |_| {
                                let mut current = pending_attachments.write();
                                current.remove(i);
                            },
                            MaterialIcon { name: "close", size: 14 }
                        }
                    }
                }
            }
        }
        div { class: "relative overflow-hidden rounded-2xl border border-slate-200/90 bg-white/94 p-2 shadow-[0_10px_30px_rgba(15,23,42,0.08)] backdrop-blur-sm transition-all focus-within:border-blue-400 focus-within:shadow-[0_12px_32px_rgba(59,130,246,0.15)]",
            div { class: "pointer-events-none absolute inset-x-3 top-0 h-px bg-gradient-to-r from-transparent via-slate-200 to-transparent" }
            div { class: "flex items-end gap-2",
                label { class: "group flex cursor-pointer items-center rounded-xl border border-slate-200 bg-slate-50 p-2.5 text-slate-400 transition-colors hover:border-slate-300 hover:bg-slate-100 hover:text-slate-600",
                    MaterialIcon { name: "attach_file", size: 24 }
                    input {
                        r#type: "file",
                        class: "hidden",
                        multiple: true,
                        onchange: move |evt| {
                            spawn(async move {
                                let files = evt.files();
                                for file in files {
                                    let name = file.name();
                                    if let Ok(contents) = file.read_bytes().await {
                                        let content_type =
                                            mime_guess::from_path(&name)
                                                .first_or_octet_stream()
                                                .to_string();
                                        pending_attachments
                                            .write()
                                            .push((name, content_type, contents.to_vec()));
                                    }
                                }
                            });
                        }
                    }
                }
                div { class: "flex-1 relative",
                    textarea {
                        class: "custom-scrollbar w-full resize-none overflow-y-auto border-0 bg-transparent px-2 py-3 text-sm leading-relaxed text-[var(--text-main)] placeholder:text-slate-400 outline-none ring-0 focus:border-0 focus:outline-none focus:ring-0 max-h-48",
                        style: "border: none; outline: none; box-shadow: none;",
                        placeholder: if (input_text)().is_empty() { "" } else { "" },
                        rows: format!("{}", (input_text().split('\n').count() as u32).min(8)),
                        value: "{input_text}",
                        oninput: move |e| input_text.set(e.value()),
                        onkeydown: move |e| {
                            if e.key() == Key::Enter && !e.modifiers().shift() {
                                e.prevent_default();
                                send.call(());
                            }
                        }
                    }
                    if (input_text)().is_empty() {
                        div { class: "pointer-events-none absolute left-2 top-3 text-sm text-slate-400 leading-relaxed",
                            AnimatedPlaceholder {
                                selected_conversation_id: selected_conversation_id
                            }
                        }
                    }
                }
                button {
                    disabled: input_text().trim().is_empty() && !is_thinking(),
                    class: format!(
                        "size-11 flex shrink-0 items-center justify-center rounded-xl transition-all duration-150 {}",
                        if is_thinking() {
                            "bg-red-500 text-white shadow-lg shadow-red-500/20 hover:-translate-y-0.5 hover:bg-red-600"
                        } else if input_text().trim().is_empty() {
                            "cursor-not-allowed bg-slate-200 text-slate-400 shadow-none"
                        } else {
                            "bg-blue-600 text-white shadow-lg shadow-blue-500/20 hover:-translate-y-0.5 hover:bg-blue-700"
                        }
                    ),
                    onclick: move |_| {
                        if is_thinking() {
                            stop.call(());
                        } else {
                            send.call(());
                        }
                    },
                    if is_thinking() {
                        MaterialIcon { name: "stop", color: MaterialIconColor::Light, size: 24 }
                    } else {
                        MaterialIcon { name: "arrow_upward", color: MaterialIconColor::Light, size: 24 }
                    }
                }
            }
        }
    }
}

#[component]
fn AnimatedPlaceholder(selected_conversation_id: Signal<Option<i64>>) -> Element {
    let mut animated_text = use_signal(|| String::new());

    use_future(move || async move {
        let new_conversation_prompts = [
            "Type anything to start a new conversation...",
            "Ask a question, plan a task, or explore an idea...",
            "Drop a request and Overseer will route it...",
        ];
        let active_conversation_prompts = [
            "Ask Overseer to analyze, automate, or explain...",
            "Request a follow-up, summary, or next step...",
            "Refine this conversation with more context...",
        ];

        let mut phrase_index = 0usize;

        loop {
            let prompts = if selected_conversation_id().is_some() {
                &active_conversation_prompts
            } else {
                &new_conversation_prompts
            };
            let phrase = prompts[phrase_index % prompts.len()];
            let chars: Vec<char> = phrase.chars().collect();

            for i in 1..=chars.len() {
                let current: String = chars.iter().take(i).collect();
                animated_text.set(current);
                tokio::time::sleep(std::time::Duration::from_millis(45)).await;
            }

            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;

            for i in (0..chars.len()).rev() {
                let current: String = chars.iter().take(i).collect();
                animated_text.set(current);
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
            }

            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            phrase_index = phrase_index.wrapping_add(1);
        }
    });

    rsx! { "{animated_text}" }
}


#[component]
fn ArtifactWorkspaceSurface(
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    starter_prompt: Signal<Option<String>>,
) -> Element {
    let artifacts = selected_artifacts();
    let actions = build_artifact_workspace_actions(&artifacts);

    rsx! {
        div { class: "mb-3 overflow-hidden rounded-2xl border border-emerald-200 bg-gradient-to-br from-emerald-50 via-white to-blue-50 shadow-sm",
            div { class: "border-b border-emerald-100 px-4 py-3",
                div { class: "flex flex-wrap items-start justify-between gap-3",
                    div { class: "space-y-1",
                        p { class: "text-[10px] font-black uppercase tracking-[0.2em] text-emerald-600", "Artifact Workspace" }
                        h3 { class: "text-sm font-black text-slate-900", "Selected artifacts will guide the next task." }
                        p { class: "text-xs text-slate-600 leading-relaxed",
                            "Use these as explicit working context instead of relying on chat memory alone."
                        }
                    }
                    button {
                        class: "inline-flex items-center gap-2 rounded-xl border border-emerald-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] text-emerald-700 hover:border-emerald-300 hover:bg-emerald-50 transition-all",
                        onclick: move |_| selected_artifacts.set(Vec::new()),
                        MaterialIcon { name: "layers_clear", size: 14 }
                        "Clear Selection"
                    }
                }
            }
            div { class: "space-y-3 px-4 py-4",
                div { class: "grid gap-3",
                    for artifact in artifacts.iter() {
                        ArtifactWorkspaceCard {
                            key: "{artifact.id}",
                            artifact: artifact.clone(),
                            selected_artifacts: selected_artifacts,
                        }
                    }
                }
                if !actions.is_empty() {
                    div { class: "space-y-2 pt-1",
                        p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Suggested Next Moves" }
                        div { class: "flex flex-wrap gap-2",
                            for (label, icon, prompt) in actions {
                                button {
                                    class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:bg-blue-50 hover:text-blue-700 transition-all",
                                    onclick: {
                                        let prompt = prompt.clone();
                                        move |_| starter_prompt.set(Some(prompt.clone()))
                                    },
                                    MaterialIcon { name: icon, size: 14 }
                                    "{label}"
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
fn ArtifactWorkspaceCard(
    artifact: backend::Artifact,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
) -> Element {
    let artifact_kind = artifact.kind.clone();
    let artifact_path = artifact.file_path.clone();
    let artifact_content_type = artifact
        .content_type
        .clone()
        .unwrap_or_else(|| "image/png".to_string());
    let image_preview = use_resource(move || {
        let kind = artifact_kind.clone();
        let path = artifact_path.clone();
        let content_type = artifact_content_type.clone();
        async move {
            if kind != "image" {
                return None;
            }
            let Some(path) = path else {
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

    let kind_label = artifact.kind.to_uppercase();
    let domain_label = artifact
        .domain
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "general".to_string())
        .replace('_', " ");
    let excerpt = artifact.content.clone().unwrap_or_default();

    rsx! {
        div { class: "rounded-2xl border border-white/80 bg-white/90 px-4 py-4 shadow-sm",
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0 space-y-1",
                    div { class: "flex flex-wrap items-center gap-2",
                        span { class: "inline-flex items-center rounded-full border border-emerald-200 bg-emerald-50 px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-emerald-700",
                            "{kind_label}"
                        }
                        span { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400",
                            "{domain_label}"
                        }
                    }
                    p { class: "text-sm font-black text-slate-900 break-words", "{artifact.label}" }
                    if let Some(step_text) = artifact.step_text.clone().filter(|value| !value.trim().is_empty()) {
                        p { class: "text-xs text-slate-500 leading-relaxed", "{step_text}" }
                    }
                }
                button {
                    class: "rounded-xl border border-slate-200 bg-slate-50 p-2 text-slate-400 hover:border-red-200 hover:bg-red-50 hover:text-red-600 transition-all",
                    onclick: {
                        let artifact_id = artifact.id;
                        move |_| selected_artifacts.write().retain(|item| item.id != artifact_id)
                    },
                    MaterialIcon { name: "close", size: 16 }
                }
            }

            if artifact.kind == "image" {
                match image_preview().flatten() {
                    Some(src) => rsx! {
                        img {
                            src: "{src}",
                            alt: "{artifact.label}",
                            class: "mt-3 max-h-48 rounded-xl border border-slate-200 bg-slate-50 object-contain"
                        }
                    },
                    None => rsx! {
                        div { class: "mt-3 rounded-xl border border-dashed border-slate-200 bg-slate-50 px-4 py-5 text-xs text-slate-400",
                            "Image preview unavailable."
                        }
                    },
                }
            } else if let Some(path) = artifact.file_path.clone().filter(|value| !value.trim().is_empty()) {
                p { class: "mt-3 text-[11px] font-mono text-slate-500 break-all", "{path}" }
            } else if !excerpt.trim().is_empty() {
                p { class: "mt-3 text-xs text-slate-600 whitespace-pre-wrap break-words line-clamp-4", "{excerpt}" }
            } else {
                p { class: "mt-3 text-xs text-slate-400", "This artifact is selected as context for the next task." }
            }
        }
    }
}

fn build_artifact_workspace_actions(
    artifacts: &[backend::Artifact],
) -> Vec<(String, String, String)> {
    if artifacts.is_empty() {
        return Vec::new();
    }

    if artifacts.len() == 1 {
        let artifact = &artifacts[0];
        let mut actions = vec![(
            "Summarize It".to_string(),
            "summarize".to_string(),
            "Summarize the selected artifact and tell me what matters most.".to_string(),
        )];

        match artifact.kind.as_str() {
            "image" => {
                actions.push((
                    "Analyze Screenshot".to_string(),
                    "image_search".to_string(),
                    "Analyze the selected screenshot and describe what is visible, important, or unusual.".to_string(),
                ));
            }
            "file" => {
                actions.push((
                    "Read File".to_string(),
                    "description".to_string(),
                    "Read the selected file and summarize its contents.".to_string(),
                ));
            }
            "record" => {
                actions.push((
                    "Extract Facts".to_string(),
                    "fact_check".to_string(),
                    "Extract the key facts from the selected artifact in a concise list."
                        .to_string(),
                ));
            }
            _ => {}
        }

        actions.push((
            "Turn Into Task".to_string(),
            "conversion_path".to_string(),
            "Use the selected artifact as context and help me decide the best next action."
                .to_string(),
        ));
        return actions;
    }

    vec![
        (
            "Compare Selected".to_string(),
            "compare".to_string(),
            "Compare the selected artifacts and explain the important differences.".to_string(),
        ),
        (
            "Synthesize Report".to_string(),
            "auto_awesome".to_string(),
            "Synthesize the selected artifacts into one clear report with the key takeaways."
                .to_string(),
        ),
        (
            "Plan Next Step".to_string(),
            "account_tree".to_string(),
            "Use the selected artifacts as context and propose the best next step.".to_string(),
        ),
    ]
}
