use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    pub class: Option<String>,
    pub conversations: Vec<backend::Conversation>,
    pub selected_conversation_id: Signal<Option<i64>>,
    pub conversations_resource: Resource<Vec<backend::Conversation>>,
    pub db: std::sync::Arc<crate::db::Database>,
}

#[component]
pub fn Sidebar(props: SidebarProps) -> Element {
    let mut system_load = use_resource(move || async move { backend::get_system_load().await });

    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            system_load.restart();
        }
    });

    let mut conversations_resource = props.conversations_resource.clone();
    let mut selected_conversation_id = props.selected_conversation_id.clone();
    let db = props.db.clone();

    let global_state = crate::state::use_global_state();
    let client = global_state.read().client.clone();

    let mut search_query = use_signal(String::new);
    let search_results = use_resource(move || {
        let client = client.clone();
        let q = search_query();
        async move {
            if q.trim().is_empty() {
                vec![]
            } else {
                client.search_messages(&q).await.unwrap_or_default()
            }
        }
    });

    rsx! {
        aside { class: format!("w-72 border-r border-slate-200 bg-white flex flex-col shrink-0 {}", props.class.as_deref().unwrap_or("")),
            div { class: "p-4 space-y-3",
                button {
                    class: "w-full flex items-center justify-center gap-2 bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2.5 rounded-xl shadow-[0_2px_8px_rgba(59,130,246,0.25)] transition-all duration-200 ease-out text-xs active:scale-[0.97] hover:shadow-[0_4px_12px_rgba(59,130,246,0.35)]",
                    onclick: {
                        let db = db.clone();
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
                div { class: "relative",
                    input {
                        class: "w-full border border-slate-200 focus:border-blue-500 focus:ring-2 focus:ring-blue-500/15 rounded-xl px-3 py-2 text-xs transition-all duration-200 ease-out outline-none placeholder:text-slate-400",
                        placeholder: "Search messages...",
                        value: "{search_query}",
                        oninput: move |evt| search_query.set(evt.value().clone())
                    }
                }
            }
            div { class: "flex-1 overflow-y-auto px-3 space-y-4",
                div {
                    if !search_query().trim().is_empty() {
                        h3 { class: "px-2 text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3",
                            "Search Results"
                        }
                        div { class: "space-y-2",
                            if let Some(results) = search_results.read().as_ref() {
                                if results.is_empty() {
                                    div { class: "px-3 py-2 text-xs italic text-slate-400", "No matches found" }
                                } else {
                                    for r in results {
                                        button {
                                            class: "w-full text-left px-3 py-2 rounded-lg border border-slate-100 hover:bg-slate-50 transition-all flex flex-col gap-1",
                                            onclick: {
                                                let id = r.conversation_id;
                                                move |_| selected_conversation_id.set(Some(id))
                                            },
                                            span { class: "text-[10px] font-bold text-blue-600 truncate uppercase tracking-wider", "{r.conversation_title}" }
                                            span { class: "text-xs text-slate-600 line-clamp-2 leading-relaxed", "{r.content}" }
                                        }
                                    }
                                }
                            } else {
                                div { class: "px-3 py-2 text-xs text-slate-400", "Searching..." }
                            }
                        }
                    } else {
                        h3 { class: "px-2 text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3",
                            "Recent History"
                        }
                        div { class: "space-y-1",
                            for conv in &props.conversations {
                            div {
                                class: format!(
                                    "w-full flex items-center gap-2 px-3 py-2 rounded-xl border transition-all duration-150 ease-out group {}",
                                    if selected_conversation_id() == Some(conv.id) {
                                        "bg-blue-50 text-blue-700 border-blue-100 shadow-[0_1px_3px_rgba(59,130,246,0.08)]"
                                    } else {
                                        "hover:bg-slate-50 text-slate-600 border-transparent hover:border-slate-200"
                                    }
                                ),
                                button {
                                    class: "flex flex-1 min-w-0 items-center gap-3 text-left",
                                    onclick: {
                                        let id = conv.id;
                                        move |_| selected_conversation_id.set(Some(id))
                                    },
                                    MaterialIcon { name: "chat_bubble", size: 18 }
                                    span { class: "text-xs font-medium truncate text-left flex-1",
                                        "{conv.title}"
                                    }
                                }
                                button {
                                    class: "shrink-0 p-1 text-slate-300 opacity-0 group-hover:opacity-100 hover:text-red-500 transition-all duration-150",
                                    title: "Delete conversation",
                                    onclick: {
                                        let db = db.clone();
                                        let id = conv.id;
                                        let current_selected = selected_conversation_id();
                                        let fallback_selected = props
                                            .conversations
                                            .iter()
                                            .find(|candidate| candidate.id != id)
                                            .map(|candidate| candidate.id);
                                        move |_| {
                                            let db = db.clone();
                                            spawn(async move {
                                                if backend::delete_conversation(&db, id).await.is_ok() {
                                                    conversations_resource.restart();
                                                    if current_selected == Some(id) {
                                                        selected_conversation_id.set(fallback_selected);
                                                    }
                                                }
                                            });
                                        }
                                    },
                                    MaterialIcon { name: "delete", size: 16 }
                                }
                            }
                        }
                    }
                    }
                }
            }
            div { class: "p-4 bg-slate-50 border-t border-slate-200",
                div { class: "bg-white rounded-lg p-3 border border-slate-200 shadow-sm",
                    if let Some(load) = system_load.value()() {
                        div { class: "flex items-center justify-between mb-2",
                            span { class: "text-[10px] font-bold text-slate-500 uppercase",
                                "{load.label}"
                            }
                            span { class: "text-[10px] font-mono text-blue-600 font-bold",
                                "{load.usage}%"
                            }
                        }
                        div { class: "h-1.5 w-full bg-slate-100 rounded-full overflow-hidden",
                            div {
                                class: "h-full bg-blue-500 transition-all duration-500",
                                style: format!("width: {}%", load.usage)
                            }
                        }
                    } else {
                        div { class: "flex items-center justify-center py-2",
                            div { class: "size-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" }
                        }
                    }
                }
            }
        }
    }
}
