use dioxus::prelude::*;
use crate::state::use_global_state;

mod execution_log;
mod control_center;
mod supervisor_panel;

#[component]
pub fn ComputerControl() -> Element {
    let state = use_global_state();
    let db = state.read().db.clone();
    
    let mut settings = use_resource({
        let db = db.clone();
        move || {
            let db = db.clone();
            async move { crate::backend::get_all_settings(&db).await }
        }
    });

    let values = settings.read().cloned().unwrap_or_default();
    let pending_action = values.get("computer_pending_action").cloned().unwrap_or_default();
    let pending_tool = values.get("computer_pending_tool").cloned().unwrap_or_default();
    let session_state = values.get("computer_session_state").cloned().unwrap_or_else(|| "idle".to_string());
    let approval_state = values.get("computer_approval_state").cloned().unwrap_or_else(|| "idle".to_string());
    
    rsx! {
        div { class: "flex-1 flex flex-col h-screen bg-slate-900 text-slate-100 overflow-hidden font-sans",
            // Top search and header bar
            div { class: "h-14 border-b border-slate-800 flex items-center justify-between px-6 bg-slate-950",
                div { class: "flex items-center gap-3",
                    span { class: "text-lg font-black tracking-wider bg-gradient-to-r from-blue-400 to-indigo-400 bg-clip-text text-transparent uppercase", "Personal AI" }
                    div { class: "relative flex items-center bg-slate-900 border border-slate-800 rounded-xl px-3 py-1.5 w-80 group",
                        dioxus_material_icons::MaterialIcon { name: "search", size: 16, color: dioxus_material_icons::MaterialIconColor::Custom("#64748b".into()) }
                        input {
                            class: "bg-transparent border-0 outline-none text-xs text-slate-300 ml-2 placeholder:text-slate-500 w-full",
                            placeholder: "Search commands or sessions..."
                        }
                    }
                }
                div { class: "flex items-center gap-4 text-xs font-bold text-slate-400",
                    div { class: "flex items-center gap-2 bg-slate-900 border border-slate-800 rounded-lg px-2 py-1",
                        span { class: "w-1.5 h-1.5 rounded-full bg-blue-500 animate-pulse" }
                        "ADMIN NODE"
                        span { class: "bg-blue-500/10 text-blue-400 text-[10px] px-1 rounded", "PRO" }
                    }
                    dioxus_material_icons::MaterialIcon { name: "notifications", size: 18 }
                    dioxus_material_icons::MaterialIcon { name: "help_outline", size: 18 }
                }
            }

            // 3-Column main content
            div { class: "flex-1 flex overflow-hidden min-h-0",
                execution_log::ExecutionLog {}
                control_center::ControlCenter {
                    session_state: session_state.clone(),
                }
                supervisor_panel::SupervisorPanel {
                    db: db.clone(),
                    pending_action: pending_action.clone(),
                    pending_tool: pending_tool.clone(),
                    approval_state: approval_state.clone(),
                    on_refresh: move || { settings.restart(); },
                }
            }
        }
    }
}
