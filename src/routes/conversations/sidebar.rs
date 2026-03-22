use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
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

    rsx! {
        aside { class: "w-72 border-r border-slate-200 bg-white flex flex-col shrink-0",
            div { class: "p-5",
                button {
                    class: "w-full flex items-center justify-center gap-2 bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2.5 rounded-lg shadow-soft transition-all text-xs",
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
            }
            div { class: "flex-1 overflow-y-auto px-4 space-y-6",
                div {
                    h3 { class: "px-2 text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3",
                        "Recent History"
                    }
                    div { class: "space-y-1",
                        for conv in &props.conversations {
                            button {
                                class: format!(
                                    "w-full flex items-center gap-3 px-3 py-2 rounded-lg border transition-all {}",
                                    if selected_conversation_id() == Some(conv.id) {
                                        "bg-blue-50 text-blue-700 border-blue-100"
                                    } else {
                                        "hover:bg-slate-50 text-slate-600 border-transparent"
                                    }
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
