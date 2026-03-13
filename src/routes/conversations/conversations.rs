use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::backend;

#[component]
pub fn Conversations() -> Element {
    let mut global_state = crate::state::use_global_state();
    let global_read = global_state.read();
    let db = global_read.db.clone();
    let agent_system = global_read.agent_system.clone();
    drop(global_read);
    
    let db_for_conv = db.clone();
    let mut conversations_resource = use_resource(move || {
        let db = db_for_conv.clone();
        async move {
            backend::get_conversations(&db).await.unwrap_or_default()
        }
    });

    let conversations = conversations_resource.read().cloned().unwrap_or_default();
    let mut selected_conversation_id = use_signal(|| conversations.first().map(|c| c.id));
    
    let db_for_msg = db.clone();
    let mut messages_resource = use_resource(move || {
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
    let mut input_text = use_signal(|| String::new());
    let mut pending_attachments = use_signal(|| Vec::<(String, String, Vec<u8>)>::new()); // (filename, content_type, data)

    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 relative",
            header { class: "h-16 shrink-0 border-b border-slate-200 bg-white/80 backdrop-blur-md flex items-center justify-between px-8 z-10",
                div { class: "flex items-center gap-6",
                    h1 { class: "text-xl font-bold tracking-tight text-[var(--text-main)]",
                        "Overseer"
                    }
                    div { class: "flex items-center gap-2 px-3 py-1 bg-slate-100 border border-slate-200 rounded-full",
                        div { class: "size-2 bg-emerald-500 rounded-full" }
                        span { class: "text-[10px] font-bold text-slate-500 uppercase tracking-widest",
                            "Llama 3 Local"
                        }
                    }
                }
                div { class: "flex items-center gap-4",
                    div { class: "flex items-center gap-2 text-slate-500 text-[11px] font-medium uppercase tracking-wider",
                        MaterialIcon { name: "storage", size: 16 }
                        span { "Knowledge: Local Repo" }
                    }
                    div { class: "h-4 w-px bg-slate-200 mx-2" }
                    button { class: "text-slate-400 hover:text-slate-600 transition-colors flex items-center",
                        MaterialIcon { name: "search", size: 24 }
                    }
                    button { class: "text-slate-400 hover:text-slate-600 transition-colors flex items-center",
                        MaterialIcon { name: "notifications", size: 24 }
                    }
                }
            }
            div { class: "flex flex-1 overflow-hidden",
                aside { class: "w-72 border-r border-slate-200 bg-white flex flex-col shrink-0",
                    div { class: "p-5",
                        button { 
                            class: "w-full flex items-center justify-center gap-2 bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2.5 rounded-lg shadow-soft transition-all text-xs",
                            onclick: {
                                let db = db.clone();
                                let mut conversations_resource = conversations_resource.clone();
                                let mut selected_conversation_id = selected_conversation_id.clone();
                                move |_| {
                                    let db = db.clone();
                                    spawn(async move {
                                        if let Ok(id) = backend::add_conversation(&db, "New Conversation").await {
                                            conversations_resource.restart();
                                            selected_conversation_id.set(Some(id));
                                        }
                                    });
                                }
                            },
                            MaterialIcon { name: "add", color: MaterialIconColor::Light, size: 20 }
                            span { "New Conversation" }
                        }
                    }
                    div { class: "flex-1 overflow-y-auto px-4 space-y-6",
                        div {
                            h3 { class: "px-2 text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3",
                                "Recent History"
                            }
                            div { class: "space-y-1",
                                for conv in conversations {
                                    button {
                                        class: format!("w-full flex items-center gap-3 px-3 py-2 rounded-lg border transition-all {}", 
                                            if selected_conversation_id() == Some(conv.id) { "bg-blue-50 text-blue-700 border-blue-100" } 
                                            else { "hover:bg-slate-50 text-slate-600 border-transparent" }
                                        ),
                                        onclick: {
                                            let id = conv.id;
                                            move |_| selected_conversation_id.set(Some(id))
                                        },
                                        MaterialIcon { name: "chat_bubble", size: 20 }
                                        span { class: "text-xs font-semibold truncate text-left flex-1",
                                            "{conv.title}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "p-4 bg-slate-50 border-t border-slate-200",
                        div { class: "bg-white rounded-lg p-3 border border-slate-200 shadow-sm",
                            div { class: "flex items-center justify-between mb-2",
                                span { class: "text-[10px] font-bold text-slate-500 uppercase",
                                    "GPU Load"
                                }
                                span { class: "text-[10px] font-mono text-blue-600 font-bold",
                                    "42%"
                                }
                            }
                            div { class: "h-1.5 w-full bg-slate-100 rounded-full overflow-hidden",
                                div { class: "h-full bg-blue-500 w-[42%]" }
                            }
                        }
                    }
                }
                main { class: "flex-1 flex flex-col relative bg-[var(--workspace-bg)]",
                    div { class: "flex-1 overflow-y-auto",
                        div { class: "max-w-3xl mx-auto py-10 px-6 space-y-8",
                            for msg in messages {
                                if msg.role == "user" {
                                    div { class: "flex items-start gap-4 flex-row-reverse",
                                        div { class: "size-8 shrink-0 rounded-lg border border-slate-200 bg-white flex items-center justify-center overflow-hidden shadow-soft",
                                            img {
                                                alt: "User Avatar",
                                                class: "w-full h-full object-cover",
                                                src: "https://lh3.googleusercontent.com/aida-public/AB6AXuBqMBjxuGBSeYlBYQz3ehCFqzRwa9Wyq9eVwMYy09GJ935PrL_9J9Nm026Ma8JaIJeKfVWN7hdeIFL0gi4M4lOputd5eqoppoLRMUGbZnFachbwDjykRgdxvZCEKJojPy5zMgRnDuAjxuLndEN_6WYnHU9MPtepRB2SOYmZ_WbKPgUjNyE5Z8A9NuRt-cff_feV6mAy2VkbTR38GeWrNCj20LzICJCLsrMqsI68pBlMyS4UV1ORb1_pYK5tyDwjtOlKMwPP2uuBrtw",
                                            }
                                        }
                                        div { class: "space-y-2 flex-1 text-right",
                                            div { class: "flex items-center gap-2 justify-end",
                                                span { class: "text-[11px] font-bold uppercase tracking-widest text-slate-500",
                                                    "Executive User"
                                                }
                                            }
                                            div { class: "bg-[var(--user-bubble)] p-4 rounded-2xl rounded-tr-none text-[var(--text-main)] text-sm leading-relaxed shadow-bubble border border-slate-200 inline-block text-left max-w-[85%]",
                                                p {
                                                    "{msg.content}"
                                                }
                                                if !msg.attachments.is_empty() {
                                                    div { class: "mt-3 flex flex-wrap gap-2 justify-end",
                                                        for att in msg.attachments {
                                                            div { class: "flex items-center gap-2 bg-white/20 px-2 py-1 rounded text-[10px] font-bold text-white border border-white/10",
                                                                MaterialIcon { name: "insert_drive_file", size: 14 }
                                                                "{att.filename}"
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    div { class: "flex items-start gap-4",
                                        div { class: "size-8 shrink-0 rounded-lg bg-blue-600 flex items-center justify-center text-white shadow-soft",
                                            MaterialIcon { name: "smart_toy", color: MaterialIconColor::Light, size: 20 }
                                        }
                                        div { class: "space-y-2 flex-1",
                                            div { class: "flex items-center gap-2",
                                                span { class: "text-[11px] font-bold uppercase tracking-widest text-slate-500",
                                                    "Overseer"
                                                }
                                                span { class: "text-[9px] bg-white text-blue-600 border border-blue-100 px-1.5 py-0.5 rounded font-bold uppercase tracking-tighter",
                                                    "SECURE"
                                                }
                                            }
                                            div { class: "bg-[var(--assistant-bubble)] p-4 rounded-2xl rounded-tl-none border border-blue-50/50 text-[var(--text-main)] text-sm leading-relaxed shadow-bubble",
                                                p {
                                                    "{msg.content}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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
                                                    if let Some(file_engine) = evt.files() {
                                                        let files = file_engine.files();
                                                        for file in files {
                                                            if let Some(contents) = file_engine.read_file(&file).await {
                                                                let content_type = mime_guess::from_path(&file).first_or_octet_stream().to_string();
                                                                pending_attachments.write().push((file.clone(), content_type, contents));
                                                            }
                                                        }
                                                    }
                                                });
                                            }
                                        }
                                    }
                                    textarea {
                                        class: "flex-1 bg-transparent border-none focus:ring-0 text-[var(--text-main)] placeholder:text-slate-400 text-sm py-3 px-2 resize-none max-h-48",
                                        placeholder: "Command assistant...",
                                        rows: "1",
                                        value: "{input_text}",
                                        oninput: move |e| input_text.set(e.value()),
                                        onkeydown: {
                                            let db_outer = db.clone();
                                            let agent_outer = agent_system.clone();
                                            let messages_outer = messages_resource.clone();
                                            let selected_outer = selected_conversation_id.clone();
                                            move |e| {
                                                if e.key() == Key::Enter && !e.modifiers().shift() {
                                                    let val = input_text();
                                                    if !val.trim().is_empty() {
                                                        input_text.set(String::new());
                                                        let attachments = pending_attachments();
                                                        pending_attachments.set(vec![]);
                                                        
                                                        let db_inner = db_outer.clone();
                                                        let agent_inner = agent_outer.clone();
                                                        let mut messages_inner = messages_outer.clone();
                                                        let conv_id = selected_outer();
                                                        let query = val.clone();
                                                        spawn(async move {
                                                            if let Some(id) = conv_id {
                                                                if let Ok(msg_id) = backend::add_message(&db_inner, id, "user", &query).await {
                                                                    for (name, mime, data) in attachments {
                                                                        let folder = "overseer_files";
                                                                        let _ = std::fs::create_dir_all(folder);
                                                                        let path = format!("{}/{}", folder, name);
                                                                        let _ = std::fs::write(&path, &data);
                                                                        let _ = backend::add_attachment(&db_inner, msg_id, &name, &path, &mime, data.len() as i64).await;
                                                                    }
                                                                    messages_inner.restart();
                                                                    
                                                                    let response = agent_inner.orchestrate(&db_inner, id, &query).await;
                                                                    let _ = backend::add_message(&db_inner, id, "assistant", &response).await;
                                                                    messages_inner.restart();
                                                                }
                                                            }
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    button { 
                                        class: "size-11 flex items-center justify-center bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-all shadow-lg shadow-blue-500/20",
                                        onclick: {
                                            let db_outer = db.clone();
                                            let agent_outer = agent_system.clone();
                                            let messages_outer = messages_resource.clone();
                                            let selected_outer = selected_conversation_id.clone();
                                            move |_| {
                                                let val = input_text();
                                                if !val.trim().is_empty() {
                                                    input_text.set(String::new());
                                                    let attachments = pending_attachments();
                                                    pending_attachments.set(vec![]);

                                                    let db_inner = db_outer.clone();
                                                    let agent_inner = agent_outer.clone();
                                                    let mut messages_inner = messages_outer.clone();
                                                    let conv_id = selected_outer();
                                                    let query = val.clone();
                                                    spawn(async move {
                                                            if let Some(id) = conv_id {
                                                                if let Ok(msg_id) = backend::add_message(&db_inner, id, "user", &query).await {
                                                                    for (name, mime, data) in attachments {
                                                                        let folder = "overseer_files";
                                                                        let _ = std::fs::create_dir_all(folder);
                                                                        let path = format!("{}/{}", folder, name);
                                                                        let _ = std::fs::write(&path, &data);
                                                                        let _ = backend::add_attachment(&db_inner, msg_id, &name, &path, &mime, data.len() as i64).await;
                                                                    }
                                                                    messages_inner.restart();
                                                                    
                                                                    let response = agent_inner.orchestrate(&db_inner, id, &query).await;
                                                                    let _ = backend::add_message(&db_inner, id, "assistant", &response).await;
                                                                    messages_inner.restart();
                                                                }
                                                            }
                                                    });
                                                }
                                            }
                                        },
                                        MaterialIcon { name: "arrow_upward", color: MaterialIconColor::Light, size: 24 }
                                    }
                                }
                            }
                            div { class: "mt-4 flex items-center justify-center gap-4",
                                span { class: "text-[10px] text-slate-400 font-bold uppercase tracking-widest",
                                    "Model: Llama-3-8B-Q4_K_M"
                                }
                                div { class: "h-1 w-1 rounded-full bg-slate-300" }
                                span { class: "text-[10px] text-slate-400 font-bold uppercase tracking-widest",
                                    "Latency: 24ms"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}