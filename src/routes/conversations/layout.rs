// src/routes/conversations/layout.rs
use crate::backend;
use super::state::ConversationsState;
use super::chip::ChatScreenChip;
use dioxus::prelude::*;

#[component]
pub fn ConversationsHeader(
    state: ConversationsState,
    conversation_title: String,
    route_reason: Option<String>,
    route_label: Option<String>,
    personality_label: Option<String>,
    latest_run: Option<backend::ExecutionRun>,
    selected_run: Option<backend::ExecutionRun>,
    is_first_msg: bool,
) -> Element {
    let global_state = crate::state::use_global_state();
    let client = global_state.read().client.clone();
    let client_for_md = client.clone();
    let client_for_json = client.clone();
    let title_for_md = conversation_title.clone();
    let title_for_json = conversation_title.clone();
    let id_opt = (state.selected_conversation_id)();

    rsx! {
        if !is_first_msg {
        div { class: "border-b border-slate-200 bg-white/90 backdrop-blur-sm px-3 sm:px-6 py-3",
            div { class: "max-w-3xl mx-auto flex items-center justify-between gap-4",
                div { class: "min-w-0",
                    h1 { class: "text-base font-semibold text-slate-900 truncate", "{conversation_title}" }
                    if let Some(reason) = route_reason {
                        p { class: "text-xs text-slate-400 mt-0.5 truncate", "{reason}" }
                    }
                }
                div { class: "flex flex-wrap items-center justify-end gap-2 shrink-0",
                    if let Some(id) = id_opt {
                        button {
                            class: "border border-slate-200 bg-white hover:bg-slate-50 text-slate-700 text-[10px] rounded-full px-3 py-1 font-black uppercase tracking-[0.18em] transition flex items-center gap-1.5 cursor-pointer",
                            onclick: move |_| {
                                let client = client_for_md.clone();
                                let title_slug = title_for_md.to_lowercase().replace(" ", "_");
                                spawn(async move {
                                    if let Ok(content) = client.export_conversation_markdown(id).await {
                                        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                        let filename = format!("{}_{}.md", title_slug, ts);
                                        if let Ok(path) = client.export_to_downloads(&filename, &content).await {
                                            let _ = client.open_local_path(&path).await;
                                        }
                                    }
                                });
                            },
                            "Export MD"
                        }
                        button {
                            class: "border border-slate-200 bg-white hover:bg-slate-50 text-slate-700 text-[10px] rounded-full px-3 py-1 font-black uppercase tracking-[0.18em] transition flex items-center gap-1.5 cursor-pointer",
                            onclick: move |_| {
                                let client = client_for_json.clone();
                                let title_slug = title_for_json.to_lowercase().replace(" ", "_");
                                spawn(async move {
                                    if let Ok(content) = client.export_conversation_json(id).await {
                                        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
                                        let filename = format!("{}_{}.json", title_slug, ts);
                                        if let Ok(path) = client.export_to_downloads(&filename, &content).await {
                                            let _ = client.open_local_path(&path).await;
                                        }
                                    }
                                });
                            },
                            "Export JSON"
                        }
                    }
                    if let Some(label) = route_label { ChatScreenChip { tone: "blue".to_string(), label: label } }
                    if let Some(label) = personality_label { ChatScreenChip { tone: "violet".to_string(), label: label } }
                    if let Some(run) = latest_run { ChatScreenChip { tone: super::route_summary::status_tone(&run.status).to_string(), label: format!("Run {}", super::route_summary::prettify_status(&run.status)) } }
                    else { ChatScreenChip { tone: "slate".to_string(), label: "Trace idle".to_string() } }
                }
            }
            if let Some(run) = selected_run {
                if let Some(error_message) = run.error_message.filter(|v| !v.trim().is_empty()) {
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
        }
    }
}
