use dioxus::prelude::*;
use crate::backend;

#[component]
pub fn SingleWidgetLayout(
    has_log: bool,
    has_center: bool,
    has_load: bool,
    has_gpu: bool,
    has_bg: bool,
    has_env: bool,
    is_listening: bool,
    run_status: String,
    events: Vec<backend::ExecutionEvent>,
    mut settings: Resource<std::collections::HashMap<String, String>>,
) -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col h-full overflow-hidden",
            div { class: "h-[62%] w-full p-6 border-b border-slate-200 bg-white overflow-y-auto custom-scrollbar flex flex-col justify-center",
                if has_log { crate::routes::computer_control::interaction_log::InteractionLog { events: events.clone() } }
                else if has_center { crate::routes::computer_control::control_center::ControlCenter { run_status: run_status.clone(), events: events.clone() } }
                else if has_load { crate::routes::computer_control::widgets_panel::SystemLoadWidget {} }
                else if has_gpu { crate::routes::computer_control::widgets_panel::GpuPerformanceWidget {} }
                else if has_bg { crate::routes::computer_control::widgets_panel::BackgroundProcessesWidget {} }
                else if has_env { crate::routes::computer_control::widgets_panel::EnvironmentWidget {} }
            }
            div { class: "h-[38%] w-full flex bg-slate-50/50",
                div { class: "w-1/2 border-r border-slate-200 p-6",
                    crate::routes::computer_control::voice_header::OverseerBottomWidget { is_listening, on_refresh: move || { settings.restart(); } }
                }
                div { class: "w-1/2 p-6 bg-white",
                    crate::routes::computer_control::voice_header::TelemetrySummary { uptime: "14h 22m".to_string(), session_id: "AC-9921".to_string() }
                }
            }
        }
    }
}
