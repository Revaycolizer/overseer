use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use crate::routes::settings::intelligence_model::IntelligenceModel;
use crate::routes::settings::intelligence_limits::IntelligenceLimits;

#[component]
pub fn IntelligenceSection(
    default_chat_model: String,
    title_model: String,
    max_iterations: String,
    tool_timeout_ms: String,
    model_timeout_ms: String,
    available_models: Vec<String>,
    on_change: EventHandler<()>,
) -> Element {
    rsx! {
        section {
            div { class: "flex items-center gap-3 mb-8",
                div { class: "p-2 bg-blue-100 rounded-lg text-blue-600",
                    MaterialIcon { name: "psychology", size: 24 }
                }
                div {
                    h3 { class: "text-xl font-bold text-slate-900", "Intelligence Engine" }
                    p { class: "text-sm text-slate-500", "Configure models and execution limits." }
                }
            }

            div { class: "grid gap-6 xl:grid-cols-2",
                IntelligenceModel {
                    default_chat_model,
                    title_model,
                    available_models,
                    on_change
                }
                IntelligenceLimits {
                    max_iterations,
                    tool_timeout_ms,
                    model_timeout_ms,
                    on_change
                }
            }
        }
    }
}
