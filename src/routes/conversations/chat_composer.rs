use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use super::animated_placeholder::AnimatedPlaceholder;
use super::artifact_workspace::ArtifactWorkspaceSurface;
use super::chat_input::ChatInputProps;

#[component]
pub fn ChatComposer(props: ChatInputProps) -> Element {
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

        if let Some(cmd) = crate::engine::commands::parse(&val) {
            is_local_executing.set(false);
            let client = global_state.read().client.clone();
            let mut conversations_resource = conversations_resource.clone();
            let mut selected_conversation_id = selected_conversation_id.clone();
            let conv_id = selected_conversation_id();
            match cmd {
                crate::engine::commands::SlashCommand::Help => {
                    if let Some(id) = conv_id {
                        visible_messages.with_mut(|messages| {
                            messages.push(backend::Message {
                                id: -1,
                                conversation_id: id,
                                role: "assistant".to_string(),
                                content: r#"{"messages":[{"message":"**Available Commands:**\n- `/new`: Start a new conversation\n- `/export md|json`: Export this conversation\n- `/model <name>`: Switch preferred planner model\n- `/rename <title>`: Rename this conversation\n- `/regen`: Populates last user message and strips last response\n- `/clear`: Clear text input\n- `/help`: Show this helper","role":"assistant"}]}"#.to_string(),
                                created_at: chrono::Utc::now().naive_utc(),
                                attachments: vec![],
                                hide_evidence: false,
                            });
                        });
                    }
                }
                crate::engine::commands::SlashCommand::NewConversation => {
                    spawn(async move {
                        if let Ok(id) = client.add_conversation("New Conversation").await {
                            conversations_resource.restart();
                            selected_conversation_id.set(Some(id));
                        }
                    });
                }
                crate::engine::commands::SlashCommand::Export { format } => {
                    if let Some(id) = conv_id {
                        spawn(async move {
                            let content = match format.as_str() {
                                "md" => client.export_conversation_markdown(id).await,
                                "json" => client.export_conversation_json(id).await,
                                _ => return,
                            };
                            if let Ok(content) = content {
                                let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                let filename = format!("conv_{}_{}.{}", id, ts, format);
                                if let Ok(path) = client.export_to_downloads(&filename, &content).await {
                                    let _ = client.open_local_path(&path).await;
                                }
                            }
                        });
                    }
                }
                crate::engine::commands::SlashCommand::Search { query: _ } => {}
                crate::engine::commands::SlashCommand::Model { name } => {
                    spawn(async move {
                        let _ = client.set_setting("planner_model", &name).await;
                    });
                }
                crate::engine::commands::SlashCommand::Regenerate => {
                    let last_user = visible_messages.read().iter().filter(|m| m.role == "user").last().cloned();
                    if let Some(user_msg) = last_user {
                        let last_ast_id = {
                            let msgs = visible_messages.read();
                            if let Some(last_ast) = msgs.last() {
                                if last_ast.role == "assistant" {
                                    Some(last_ast.id)
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };
                        if let Some(msg_id) = last_ast_id {
                            visible_messages.with_mut(|messages| {
                                messages.pop();
                            });
                            spawn(async move {
                                let _ = client.delete_message(msg_id).await;
                            });
                        }
                        input_text.set(user_msg.content);
                    }
                }
                crate::engine::commands::SlashCommand::Rename { title } => {
                    if let Some(id) = conv_id {
                        spawn(async move {
                            if client.update_conversation_title(id, &title).await.is_ok() {
                                conversations_resource.restart();
                            }
                        });
                      }
                }
                crate::engine::commands::SlashCommand::Clear => {
                    input_text.set(String::new());
                }
                crate::engine::commands::SlashCommand::Quit => {}
            }
            return;
        }

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

            // No 4-word optimistic title write: the LLM refinement via
            // the chat_input spawn (and the engine_system.orchestrate
            // one) writes the proper title within 1-3 s. The 4-word
            // placeholder was racing with the LLM and could overwrite
            // a real title when the LLM returned empty / errored.
            let spawn_llm_refinement = is_first_msg_val || created_new_conversation;

            let (title_tx, title_rx) = tokio::sync::oneshot::channel::<()>();
            let mut conversations_for_title = conversations_inner.clone();
            spawn(async move {
                if title_rx.await.is_ok() {
                    conversations_for_title.restart();
                }
            });

            let assistant_tx = crate::routes::conversations::assistant_stream::install_listener(visible_messages);

            // V4.17.1d: per-token streaming. The streaming consumer must
            // be a Dioxus spawn (not a tokio spawn) because it captures
            // the non-Send `visible_messages` Signal. It runs alongside
            // the outer tokio spawn (which only holds Send values).
            let (token_tx, token_rx) =
                tokio::sync::mpsc::unbounded_channel::<String>();
            let mut visible_for_stream = visible_messages;
            spawn(async move {
                let mut partial = String::new();
                let mut receiver = token_rx;
                while let Some(chunk) = receiver.recv().await {
                    partial.push_str(&chunk);
                    visible_for_stream.with_mut(|msgs| {
                        if let Some(last) = msgs.last_mut() {
                            if last.role == "assistant" {
                                last.content = partial.clone();
                            } else {
                                msgs.push(backend::Message {
                                    id: 0,
                                    conversation_id,
                                    role: "assistant".to_string(),
                                    content: partial.clone(),
                                    created_at: chrono::Utc::now().naive_utc(),
                                    attachments: vec![],
                                    hide_evidence: false,
                                });
                            }
                        }
                    });
                }
            });

            let handle = tokio::spawn(async move {
                if let Ok(mut cache) = crate::engine::cache::messages().write() {
                    cache.invalidate(&conversation_id);
                }
                if let Ok(msg_id) = backend::add_message(
                    &db_inner,
                    conversation_id,
                    "user",
                    &query_with_artifacts,
                    false,
                )
                .await
                {
                    if spawn_llm_refinement {
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

                    // V4.17.1d: per-token streaming. The streaming
                    // consumer is a Dioxus spawn above this outer
                    // tokio::spawn (set up at line ~351). The dispatch's
                    // collect_stream_with_sender pushes each text
                    // chunk to a tokio::mpsc::UnboundedSender; the
                    // Dioxus consumer drains the receiver and updates
                    // the partial assistant message in
                    // visible_messages in real time. Here we just pass
                    // the sender into orchestrate.
                    let response = agent_inner
                        .orchestrate(
                            &db_inner,
                            conversation_id,
                            &query_with_artifacts,
                            Some(token_tx),
                        )
                        .await;

                    let _ =
                        backend::add_message(&db_inner, conversation_id, "assistant", &response, false)
                            .await;
                    // Invalidate the cached first-page for this conversation so
                    // a switch-away-and-back returns the assistant message; the
                    // channel push above already updated the live visible_messages
                    // but the persistent cache only saw the user message.
                    if let Ok(mut cache) = crate::engine::cache::messages().write() {
                        cache.invalidate(&conversation_id);
                    }
                    let _ = assistant_tx.send(crate::routes::conversations::assistant_stream::assistant_message(conversation_id, response.clone()));
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
            if input_text().starts_with('/') {
                if !crate::engine::commands::get_suggestions(&input_text()).is_empty() {
                    div { class: "flex items-center gap-1.5 px-3 py-1.5 border-b border-slate-100 overflow-x-auto select-none",
                        span { class: "text-[9px] font-black text-slate-400 uppercase tracking-widest mr-1 shrink-0", "Suggestions" }
                        for cmd in crate::engine::commands::get_suggestions(&input_text()) {
                            button {
                                class: "px-2 py-0.5 rounded bg-slate-50 hover:bg-blue-50 text-[10px] font-medium text-slate-500 hover:text-blue-700 transition cursor-pointer border border-slate-200/60 shrink-0",
                                onclick: {
                                    let cmd = cmd.to_string();
                                    move |_| {
                                        input_text.set(cmd.clone());
                                    }
                                },
                                "{cmd}"
                            }
                        }
                    }
                }
            }
            div { class: "pointer-events-none absolute inset-x-3 top-0 h-px bg-gradient-to-r from-transparent via-slate-200 to-transparent" }
            div { class: "flex items-end gap-2 mt-1",
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
                            } else if e.key() == Key::Tab {
                                if input_text().starts_with('/') {
                                    e.prevent_default();
                                    let suggestions = crate::engine::commands::get_suggestions(&input_text());
                                    if let Some(first_match) = suggestions.first() {
                                        input_text.set(first_match.to_string());
                                    }
                                }
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

use super::animated_placeholder::AnimatedPlaceholder;
use super::artifact_workspace::ArtifactWorkspaceSurface;
