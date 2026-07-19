use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use super::animated_placeholder::AnimatedPlaceholder;
use super::chat_input::ChatInputProps;
use super::chat_send::use_chat_state;

#[component]
pub fn ChatComposer(props: ChatInputProps) -> Element {
    let state = use_chat_state(&props);
    let mut input_text = state.input_text;
    let is_thinking = state.is_thinking;
    let mut pending_attachments = state.pending_attachments;
    let selected_conversation_id = props.selected_conversation_id;
    let send_fn = state.send.clone();
    let stop_fn = state.stop.clone();
    let large = props.large;

    rsx! {
        div { class: "border-t border-slate-200/80 bg-white/96 px-2 sm:px-4 pt-2 sm:pt-3 pb-3 sm:pb-5",
            if !pending_attachments().is_empty() {
                div { class: "mb-3 flex flex-wrap gap-2",
                    for (i, (name, _, _)) in pending_attachments().iter().enumerate() {
                        div { class: "flex items-center gap-2 rounded-xl border border-blue-100 bg-blue-50/80 px-3 py-2 text-[10px] font-black tracking-[0.08em] text-blue-700 shadow-sm",
                            MaterialIcon { name: "insert_drive_file", size: 16 }
                            "{name}"
                            button { class: "ml-1 rounded-md p-0.5 text-blue-400 hover:bg-blue-100 hover:text-blue-700 transition-colors",
                                onclick: move |_| { pending_attachments.write().remove(i); },
                                MaterialIcon { name: "close", size: 14 }
                            }
                        }
                    }
                }
            }
            div { class: if large { "relative overflow-hidden rounded-3xl border border-slate-200 bg-white p-3 shadow-[0_16px_44px_rgba(15,23,42,0.10)] backdrop-blur-sm transition-all focus-within:border-blue-400 focus-within:shadow-[0_18px_50px_rgba(59,130,246,0.18)]" } else { "relative overflow-hidden rounded-2xl border border-slate-200/90 bg-white/94 p-2 shadow-[0_10px_30px_rgba(15,23,42,0.08)] backdrop-blur-sm transition-all focus-within:border-blue-400 focus-within:shadow-[0_12px_32px_rgba(59,130,246,0.15)]" },
                if input_text().starts_with('/') && !crate::engine::commands::get_suggestions(&input_text()).is_empty() {
                    div { class: "flex items-center gap-1.5 px-3 py-1.5 border-b border-slate-100 overflow-x-auto select-none",
                        span { class: "text-[9px] font-black text-slate-400 uppercase tracking-widest mr-1 shrink-0", "Suggestions" }
                        for cmd in crate::engine::commands::get_suggestions(&input_text()) {
                            button { class: "px-2 py-0.5 rounded bg-slate-50 hover:bg-blue-50 text-[10px] font-medium text-slate-500 hover:text-blue-700 transition cursor-pointer border border-slate-200/60 shrink-0",
                                onclick: { let c = cmd.to_string(); move |_| input_text.set(c.clone()) },
                                "{cmd}"
                            }
                        }
                    }
                }
                div { class: "flex items-end gap-2 mt-1",
                    label { class: "group flex cursor-pointer items-center rounded-xl border border-slate-200 bg-slate-50 p-2.5 text-slate-400 transition-colors hover:border-slate-300 hover:bg-slate-100 hover:text-slate-600",
                        MaterialIcon { name: "attach_file", size: 24 }
                        input { r#type: "file", class: "hidden", multiple: true,
                            onchange: move |evt| { spawn(async move { for file in evt.files() { if let Ok(contents) = file.read_bytes().await { pending_attachments.write().push((file.name(), mime_guess::from_path(&file.name()).first_or_octet_stream().to_string(), contents.to_vec())); } } }); }
                        }
                    }
                    div { class: "flex-1 relative",
                        textarea { class: if large { "custom-scrollbar w-full resize-none overflow-y-auto border-0 bg-transparent px-4 py-5 text-lg leading-relaxed text-[var(--text-main)] placeholder:text-slate-400 outline-none ring-0 focus:border-0 focus:outline-none focus:ring-0 min-h-[140px] max-h-[60vh]" } else { "custom-scrollbar w-full resize-none overflow-y-auto border-0 bg-transparent px-2 py-3 text-sm leading-relaxed text-[var(--text-main)] placeholder:text-slate-400 outline-none ring-0 focus:border-0 focus:outline-none focus:ring-0 max-h-48" },
                            style: "border: none; outline: none; box-shadow: none;",
                            rows: format!("{}", (input_text().split('\n').count() as u32).min(8)),
                            value: "{input_text}",
                            oninput: move |e| input_text.set(e.value()),
                            onkeydown: move |e| {
                                if e.key() == Key::Enter && !e.modifiers().shift() { e.prevent_default(); send_fn.call(()); }
                                else if e.key() == Key::Tab && input_text().starts_with('/') { e.prevent_default(); if let Some(m) = crate::engine::commands::get_suggestions(&input_text()).first() { input_text.set(m.to_string()); } }
                            }
                        }
                        if input_text().is_empty() {
                            div { class: "pointer-events-none absolute left-2 top-3 text-sm text-slate-400 leading-relaxed",
                                AnimatedPlaceholder { selected_conversation_id }
                            }
                        }
                    }
                    button {
                        disabled: input_text().trim().is_empty() && !is_thinking(),
                        class: format!("size-11 flex shrink-0 items-center justify-center rounded-xl transition-all duration-150 {}", if is_thinking() { "bg-red-500 text-white shadow-lg shadow-red-500/20 hover:-translate-y-0.5 hover:bg-red-600" } else if input_text().trim().is_empty() { "cursor-not-allowed bg-slate-200 text-slate-400 shadow-none" } else { "bg-blue-600 text-white shadow-lg shadow-blue-500/20 hover:-translate-y-0.5 hover:bg-blue-700" }),
                        onclick: move |_| { if is_thinking() { stop_fn.call(()); } else { send_fn.call(()); } },
                        MaterialIcon { name: if is_thinking() { "stop" } else { "arrow_upward" }, size: 22, color: MaterialIconColor::Light }
                    }
                }
            }
        }
    }
}
