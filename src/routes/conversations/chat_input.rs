use crate::backend;
use crate::state::use_global_state;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[derive(Props, Clone, PartialEq)]
pub struct ChatInputProps {
    pub selected_conversation_id: Signal<Option<i64>>,
    pub messages_resource: Resource<Vec<backend::Message>>,
    pub conversations_resource: Resource<Vec<backend::Conversation>>,
    pub is_first_msg: bool,
    pub is_thinking: Signal<bool>,
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

    let mut messages_resource = props.messages_resource.clone();
    let mut conversations_resource = props.conversations_resource.clone();
    let selected_conversation_id = props.selected_conversation_id.clone();
    let is_first_msg = props.is_first_msg;

    let mut send = use_callback(move |()| {
        let val = input_text();
        if val.trim().is_empty() {
            return;
        }
        input_text.set(String::new());
        let attachments = pending_attachments();
        pending_attachments.set(vec![]);

        let db_inner = db.clone();
        let agent_inner = agent_system.clone();
        let mut messages_inner = messages_resource.clone();
        let mut conversations_inner = conversations_resource.clone();
        let conv_id = selected_conversation_id();
        let query = val.clone();

        spawn(async move {
            if let Some(id) = conv_id {
                if let Ok(msg_id) = backend::add_message(&db_inner, id, "user", &query).await {
                    if is_first_msg {
                        let new_title = agent_inner.generate_title(&db_inner, &query).await;
                        let _ = backend::update_conversation_title(&db_inner, id, &new_title).await;
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
                    let response = agent_inner.orchestrate(&db_inner, id, &query).await;
                    is_thinking.set(false);
                    let _ = backend::add_message(&db_inner, id, "assistant", &response).await;
                    messages_inner.restart();
                }
            }
        });
    });

    rsx! {
        div { class: "p-6 bg-gradient-to-t from-[var(--workspace-bg)] via-[var(--workspace-bg)] to-transparent",
            div { class: "max-w-3xl mx-auto",
                if !pending_attachments().is_empty() {
                    div { class: "mb-3 flex flex-wrap gap-2",
                        for (i, (name, _, _)) in pending_attachments().iter().enumerate() {
                            div { class: "flex items-center gap-2 bg-blue-50 px-3 py-1.5 rounded-lg border border-blue-100 text-[10px] font-bold text-blue-600",
                                MaterialIcon { name: "insert_drive_file", size: 16 }
                                "{name}"
                                button {
                                    class: "ml-1 text-blue-400 hover:text-blue-600",
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
                div { class: "relative bg-white border border-slate-200 rounded-xl p-2 shadow-xl focus-within:border-blue-400 focus-within:ring-4 focus-within:ring-blue-500/5 transition-all",
                    div { class: "flex items-end gap-2",
                        label { class: "p-2.5 text-slate-400 hover:text-slate-600 transition-colors bg-slate-50 rounded-lg border border-slate-100 group flex items-center cursor-pointer",
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
                            class: "flex-1 bg-transparent border-none focus:ring-0 text-[var(--text-main)] placeholder:text-slate-400 text-sm py-3 px-2 resize-none max-h-48 overflow-y-auto",
                            placeholder: "Command assistant...",
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
                            class: "size-11 flex items-center justify-center bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-all shadow-lg shadow-blue-500/20",
                            onclick: move |_| send.call(()),
                            MaterialIcon { name: "arrow_upward", color: MaterialIconColor::Light, size: 24 }
                        }
                    }
                }
            }
        }
    }
}
