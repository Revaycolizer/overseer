use dioxus::prelude::*;
use crate::state::use_global_state;

mod execution_log;
mod control_center;
mod supervisor_panel;
mod voice_header;
mod interaction_log;
mod widgets_panel;

#[component]
pub fn ComputerControl() -> Element {
    let mut state = use_global_state();
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
    let session_state = values.get("computer_session_state").cloned().unwrap_or_else(|| "idle".to_string());
    
    // Customize view checklist state
    let mut active_widgets = use_signal(|| vec![
        "Interaction Log",
        "Control Center",
        "System Load",
        "GPU Performance",
        "Background Processes",
        "Environment",
    ]);
    let mut show_customizer = use_signal(|| false);

    let active_list = active_widgets.read().clone();
    let active_count = active_list.len();

    let has_log = active_list.contains(&"Interaction Log");
    let has_center = active_list.contains(&"Control Center");
    let has_load = active_list.contains(&"System Load");
    let has_gpu = active_list.contains(&"GPU Performance");
    let has_bg = active_list.contains(&"Background Processes");
    let has_env = active_list.contains(&"Environment");

    let is_listening = *state.read().is_listening.read();

    rsx! {
        div { class: "flex-1 flex flex-col h-screen bg-white text-slate-800 overflow-hidden font-sans relative",
            // Header Bar
            div { class: "h-14 border-b border-slate-200 flex items-center justify-between px-6 bg-white shrink-0 relative z-30",
                div { class: "flex items-center gap-3",
                    span { class: "text-lg font-black tracking-wider bg-gradient-to-r from-blue-600 to-indigo-600 bg-clip-text text-transparent uppercase", "Personal AI" }
                    div { class: "relative flex items-center bg-slate-50 border border-slate-200 rounded-xl px-3 py-1.5 w-80 group",
                        dioxus_material_icons::MaterialIcon { name: "search", size: 16, color: dioxus_material_icons::MaterialIconColor::Custom("#94a3b8".into()) }
                        input {
                            class: "bg-transparent border-0 outline-none text-xs text-slate-700 ml-2 placeholder:text-slate-400 w-full",
                            placeholder: "Search logs, tasks, or nodes..."
                        }
                    }
                }
                
                // Customize View Checklist Dropdown
                div { class: "flex items-center gap-4 text-xs font-bold text-slate-500",
                    div { class: "relative",
                        button {
                            class: "bg-slate-50 hover:bg-slate-100 border border-slate-200 px-3.5 py-1.5 rounded-xl flex items-center gap-1.5 transition-all text-[11px] uppercase tracking-wider text-slate-700 shadow-sm",
                            onclick: move |_| show_customizer.toggle(),
                            dioxus_material_icons::MaterialIcon { name: "dashboard_customize", size: 14 }
                            "Customize View"
                        }
                        if show_customizer() {
                            div { class: "absolute right-0 mt-2 w-56 bg-white border border-slate-200 rounded-2xl p-4 shadow-xl z-50 space-y-2.5 text-xs text-slate-700",
                                p { class: "font-black text-slate-800 uppercase tracking-wider text-[10px] pb-1 border-b border-slate-100", "Toggle Widgets" }
                                for name in ["Interaction Log", "Control Center", "System Load", "GPU Performance", "Background Processes", "Environment"] {
                                    label { class: "flex items-center gap-2.5 py-1 hover:bg-slate-50 px-1 rounded cursor-pointer transition-colors",
                                        input {
                                            r#type: "checkbox",
                                            class: "accent-blue-600 w-4 h-4 rounded border-slate-300",
                                            checked: active_widgets.read().contains(&name),
                                            onchange: move |e| {
                                                let mut list = active_widgets.read().clone();
                                                if e.value() == "true" {
                                                    if !list.contains(&name) { list.push(name); }
                                                } else {
                                                    list.retain(|&x| x != name);
                                                }
                                                active_widgets.set(list);
                                            }
                                        }
                                        span { class: "font-medium", "{name}" }
                                    }
                                }
                            }
                        }
                    }

                    div { class: "flex items-center gap-2 bg-slate-50 border border-slate-200 rounded-lg px-2 py-1 text-slate-700",
                        span { class: "w-1.5 h-1.5 rounded-full bg-blue-600 animate-pulse" }
                        "ADMIN NODE"
                        span { class: "bg-blue-600/10 text-blue-600 text-[10px] px-1 rounded", "PRO" }
                    }
                    dioxus_material_icons::MaterialIcon { name: "notifications", size: 18 }
                }
            }

            // Pulsing voice listener overlay when system is listening
            voice_header::VoiceHeader {
                is_listening,
                on_terminate: move |_| {
                    state.write().is_listening.set(false);
                    state.write().is_inhibited.set(false);
                }
            }

            // Dynamic Widget Sizing Engine
            div { class: "flex-1 flex overflow-hidden min-h-0",
                if active_count == 1 {
                    // Single Widget upfront layout: Top widget fills major viewport, bottom renders Overseer + Telemetry
                    div { class: "flex-1 flex flex-col h-full overflow-hidden",
                        div { class: "h-[62%] w-full p-6 border-b border-slate-200 bg-white overflow-y-auto custom-scrollbar flex flex-col justify-center",
                            if has_log { interaction_log::InteractionLog {} }
                            else if has_center { control_center::ControlCenter { session_state: session_state.clone() } }
                            else if has_load { widgets_panel::SystemLoadWidget {} }
                            else if has_gpu { widgets_panel::GpuPerformanceWidget {} }
                            else if has_bg { widgets_panel::BackgroundProcessesWidget {} }
                            else if has_env { widgets_panel::EnvironmentWidget {} }
                        }
                        // Bottom row splits: Left Overseer audio panel, Right Telemetry card
                        div { class: "h-[38%] w-full flex bg-slate-50/50",
                            div { class: "w-1/2 border-r border-slate-200 p-6",
                                voice_header::OverseerBottomWidget {
                                    is_listening,
                                    on_refresh: move || { settings.restart(); },
                                }
                            }
                            div { class: "w-1/2 p-6 bg-white",
                                voice_header::TelemetrySummary {
                                    uptime: "14h 22m".to_string(),
                                    session_id: "AC-9921".to_string(),
                                }
                            }
                        }
                    }
                    if has_log {
                        interaction_log::InteractionLog {}
                    }
                    if has_center {
                        control_center::ControlCenter {
                            session_state: session_state.clone(),
                        }
                    }
                    if has_load || has_gpu || has_bg || has_env {
                        widgets_panel::WidgetsPanel {
                            db: db.clone(),
                            has_load,
                            has_gpu,
                            has_bg,
                            has_env,
                            pending_action: pending_action.clone(),
                            on_refresh: move || { settings.restart(); },
                        }
                    }
                }
            }
        }
    }
}
