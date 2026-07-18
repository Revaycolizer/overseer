use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn BrowserSessions() -> Element {
    let mut sessions = use_signal(Vec::<i64>::new);
    let mut status = use_signal(|| String::from("Idle"));

    rsx! {
        div { class: "flex-1 flex flex-col h-screen bg-white",
            div { class: "border-b border-slate-200 px-6 py-4",
                h1 { class: "text-lg font-bold text-slate-800", "Browser Sessions" }
                p { class: "text-xs text-slate-500 mt-0.5", "Manage remote browser instances" }
            }
            div { class: "flex-1 p-6 overflow-y-auto",
                div { class: "max-w-2xl mx-auto space-y-4",
                    div { class: "bg-blue-50 border border-blue-100 rounded-xl p-4 flex items-center justify-between",
                        div { class: "flex items-center gap-3",
                            MaterialIcon { name: "language", size: 24, color: MaterialIconColor::Custom("#2563eb".into()) }
                            div {
                                p { class: "text-sm font-semibold text-slate-800", "Obscura Browser Engine" }
                                p { class: "text-xs text-slate-500", "Status: {status}" }
                            }
                        }
                        button { class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-xl text-xs font-semibold transition-all active:scale-95",
                            onclick: move |_| {
                                let len = sessions.read().len();
                                sessions.write().push(len as i64 + 1);
                                status.set("Running".into());
                            },
                            "New Session"
                        }
                    }
                    for (i, sid) in sessions.read().iter().enumerate() {
                        div { class: "border border-slate-200 rounded-xl p-4 flex items-center justify-between hover:border-slate-300 transition-colors",
                            div { class: "flex items-center gap-3",
                                div { class: "size-8 rounded-lg bg-green-100 flex items-center justify-center",
                                    MaterialIcon { name: "check_circle", size: 18, color: MaterialIconColor::Custom("#16a34a".into()) }
                                }
                                div {
                                    p { class: "text-sm font-medium text-slate-800", "Session #{sid}" }
                                    p { class: "text-xs text-slate-400", "Port: {9300 + i as u16}" }
                                }
                            }
                            div { class: "flex gap-2",
                                button { class: "px-3 py-1.5 rounded-lg bg-slate-100 text-xs font-medium text-slate-600 hover:bg-slate-200 transition-colors", "Navigate" }
                                button { class: "px-3 py-1.5 rounded-lg bg-slate-100 text-xs font-medium text-red-600 hover:bg-red-50 transition-colors", "Close" }
                            }
                        }
                    }
                }
            }
        }
    }
}
