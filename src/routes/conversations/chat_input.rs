use crate::backend;
use crate::state::use_global_state;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[derive(Props, Clone, PartialEq)]
pub struct ChatInputProps {
    pub selected_conversation_id: Signal<Option<i64>>,
    pub messages_resource: Resource<Vec<backend::Message>>,
    pub visible_messages: Signal<Vec<backend::Message>>,
    pub conversations_resource: Resource<Vec<backend::Conversation>>,
    pub is_first_msg: bool,
    pub is_thinking: Signal<bool>,
    pub starter_prompt: Signal<Option<String>>,
}

#[component]
pub fn ChatInput(props: ChatInputProps) -> Element {
    let global_state = use_global_state();
    let global_read = global_state.read();
    let db = global_read.db.clone();
    let agent_system = global_read.agent_system.clone();
    drop(global_read);

    let mut input_text = use_signal(|| String::new());
    let mut pending_attachments = use_signal(|| Vec::<(String, String, Vec<u8>)>::new());
    let mut is_thinking = props.is_thinking;
    let mut animated_placeholder =
        use_signal(|| "Type anything to start a new conversation...".to_string());

    let mut messages_resource = props.messages_resource.clone();
    let mut visible_messages = props.visible_messages;
    let mut conversations_resource = props.conversations_resource.clone();
    let selected_conversation_id = props.selected_conversation_id.clone();
    let selected_conversation_for_placeholder = selected_conversation_id.clone();
    let is_first_msg = props.is_first_msg;
    let mut starter_prompt = props.starter_prompt;

    use_future(move || {
        let mut animated_placeholder = animated_placeholder;
        let selected_conversation_id = selected_conversation_for_placeholder;
        async move {
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
                    animated_placeholder.set(current);
                    tokio::time::sleep(std::time::Duration::from_millis(38)).await;
                }

                tokio::time::sleep(std::time::Duration::from_millis(1250)).await;

                for i in (0..chars.len()).rev() {
                    let current: String = chars.iter().take(i).collect();
                    animated_placeholder.set(current);
                    tokio::time::sleep(std::time::Duration::from_millis(24)).await;
                }

                tokio::time::sleep(std::time::Duration::from_millis(220)).await;
                phrase_index = phrase_index.wrapping_add(1);
            }
        }
    });

    use_effect(move || {
        if let Some(prompt) = starter_prompt() {
            input_text.set(prompt);
            starter_prompt.set(None);
        }
    });

    let mut send = use_callback(move |()| {
        let val = input_text();
        if val.trim().is_empty() || is_thinking() {
            return;
        }
        input_text.set(String::new());
        let attachments = pending_attachments();
        pending_attachments.set(vec![]);

        let db_inner = db.clone();
        let agent_inner = agent_system.clone();
        let mut messages_inner = messages_resource.clone();
        let mut visible_messages_inner = visible_messages;
        let mut conversations_inner = conversations_resource.clone();
        let conv_id = selected_conversation_id();
        let mut selected_conversation_id_inner = selected_conversation_id.clone();
        let query = val.clone();
        let optimistic_id = -chrono::Utc::now().timestamp_micros();

        if let Some(id) = conv_id {
            visible_messages.with_mut(|messages| {
                messages.push(backend::Message {
                    id: optimistic_id,
                    conversation_id: id,
                    role: "user".to_string(),
                    content: query.clone(),
                    created_at: chrono::Utc::now().naive_utc(),
                    attachments: vec![],
                    hide_evidence: false,
                });
            });
        }

        spawn(async move {
            let mut created_new_conversation = false;
            let conversation_id = if let Some(id) = conv_id {
                id
            } else {
                match backend::add_conversation(&db_inner, "New Conversation").await {
                    Ok(id) => {
                        created_new_conversation = true;
                        selected_conversation_id_inner.set(Some(id));
                        conversations_inner.restart();
                        visible_messages_inner.set(vec![backend::Message {
                            id: optimistic_id,
                            conversation_id: id,
                            role: "user".to_string(),
                            content: query.clone(),
                            created_at: chrono::Utc::now().naive_utc(),
                            attachments: vec![],
                            hide_evidence: false,
                        }]);
                        id
                    }
                    Err(_) => return,
                }
            };

            if let Ok(msg_id) = backend::add_message(&db_inner, conversation_id, "user", &query, false).await {
                if is_first_msg || created_new_conversation {
                    let new_title = agent_inner.generate_title(&db_inner, &query).await;
                    let _ = backend::update_conversation_title(&db_inner, conversation_id, &new_title).await;
                    conversations_inner.restart();
                }
                for (name, mime, data) in attachments {
                    let folder = "overseer_files";
                    let _ = std::fs::create_dir_all(folder);
                    let path = format!("{folder}/{name}");
                    let _ = std::fs::write(&path, &data);
                    let _ = backend::add_attachment(
                        &db_inner,
                        msg_id,
                        &name,
                        &path,
                        &mime,
                        data.len() as i64,
                    )
                    .await;
                }
                messages_inner.restart();
                is_thinking.set(true);
                let response = agent_inner.orchestrate(&db_inner, conversation_id, &query).await;
                is_thinking.set(false);
                let _ = backend::add_message(&db_inner, conversation_id, "assistant", &response, false).await;
                messages_inner.restart();
            } else {
                visible_messages_inner
                    .write()
                    .retain(|message| message.id != optimistic_id);
            }
        });
    });

    rsx! {
        div { class: "p-6 bg-gradient-to-t from-[var(--workspace-bg)] via-[var(--workspace-bg)]/95 to-transparent",
            div { class: "max-w-3xl mx-auto",
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
                div { class: "relative overflow-hidden rounded-2xl border border-slate-200/90 bg-white/95 p-2 shadow-[0_10px_30px_rgba(15,23,42,0.08)] backdrop-blur-sm transition-all focus-within:border-blue-400 focus-within:shadow-[0_12px_32px_rgba(59,130,246,0.15)]",
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
                        textarea {
                            class: "custom-scrollbar flex-1 resize-none overflow-y-auto border-0 bg-transparent px-2 py-3 text-sm leading-relaxed text-[var(--text-main)] placeholder:text-slate-400 outline-none ring-0 focus:border-0 focus:outline-none focus:ring-0 max-h-48",
                            style: "border: none; outline: none; box-shadow: none;",
                            placeholder: "{animated_placeholder}",
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
                        button {
                            disabled: input_text().trim().is_empty() || is_thinking(),
                            class: format!(
                                "size-11 flex shrink-0 items-center justify-center rounded-xl transition-all duration-150 {}",
                                if input_text().trim().is_empty() || is_thinking() {
                                    "cursor-not-allowed bg-slate-200 text-slate-400 shadow-none"
                                } else {
                                    "bg-blue-600 text-white shadow-lg shadow-blue-500/20 hover:-translate-y-0.5 hover:bg-blue-700"
                                }
                            ),
                            onclick: move |_| send.call(()),
                            if is_thinking() {
                                div { class: "size-5 border-2 border-current border-t-transparent rounded-full animate-spin" }
                            } else {
                                MaterialIcon { name: "arrow_upward", color: MaterialIconColor::Light, size: 24 }
                            }
                        }
                    }
                }
                div { class: "mt-3 flex items-center justify-between gap-3 px-1 text-[11px] text-slate-500",
                    div { class: "flex items-center gap-2 min-w-0",
                        MaterialIcon { name: "radar", size: 14 }
                        span {
                            if is_thinking() {
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
