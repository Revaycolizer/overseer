use dioxus::prelude::*;
use std::sync::Arc;
use crate::db::Database;

#[component]
pub fn WidgetsPanel(
    db: Arc<Database>,
    has_load: bool,
    has_gpu: bool,
    has_bg: bool,
    has_env: bool,
    pending_action: String,
    on_refresh: Callback<()>,
) -> Element {
    rsx! {
        div { class: "flex-1 p-6 space-y-4 overflow-y-auto custom-scrollbar",
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-4",
                if has_load { crate::routes::computer_control::widgets::SystemLoadWidget {} }
                if has_gpu { crate::routes::computer_control::widgets::GpuPerformanceWidget {} }
                if has_bg { crate::routes::computer_control::widgets::BgProcessesWidget {} }
                if has_env { crate::routes::computer_control::widgets::EnvWidget {} }
            }
        }
    }
}
