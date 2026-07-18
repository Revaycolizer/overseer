use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use crate::routes::settings::connection::ConnectionSection;
use crate::routes::settings::health_summary::HealthSummary;
use crate::routes::settings::health_details::HealthDetails;
use crate::routes::settings::intelligence::IntelligenceSection;
use crate::routes::settings::integrity::IntegritySection;

#[component]
pub fn Settings() -> Element {
    let state = crate::state::use_global_state();
    let db = state.read().db.clone();
    let settings_db = db.clone();
    let health_db = db.clone();

    let mut settings = use_resource(move || {
        let db = settings_db.clone();
        async move { crate::backend::get_all_settings(&db).await }
    });

    let mut models = use_resource(move || async move { crate::backend::get_available_models().await });
    let mut runtime_health = use_resource(move || {
        let db = health_db.clone();
        async move { crate::backend::get_runtime_health(&db).await }
    });

    let current_settings = settings.read().cloned().unwrap_or_default();
    let available_models = models.read().cloned().unwrap_or_default();
    let health = runtime_health.read().cloned();
    let first_available_model = available_models.first().cloned().unwrap_or_else(|| "qwen2.5:7b-instruct".to_string());

    let title_model = current_settings.get("title_generator_model").cloned().unwrap_or_else(|| first_available_model.clone());
    let default_chat_model = current_settings.get("default_chat_model").cloned().unwrap_or_else(|| first_available_model.clone());
    let max_iterations = current_settings.get("max_iterations").cloned().unwrap_or_else(|| "10".to_string());
    let tool_timeout_ms = current_settings.get("tool_timeout_ms").cloned().unwrap_or_else(|| "45000".to_string());
    let model_timeout_ms = current_settings.get("model_timeout_ms").cloned().unwrap_or_else(|| "90000".to_string());
    let idle_lock_timeout_secs = current_settings.get("idle_lock_timeout_secs").cloned().unwrap_or_else(|| "1800".to_string());
    
    let runtime_dir = crate::engine::utils::discovery::runtime_dir().display().to_string();
    let release_version = env!("CARGO_PKG_VERSION");

    let on_change = move |_| {
        settings.restart();
    };

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0 bg-slate-50 overflow-y-auto custom-scrollbar",
            main { class: "flex-1 w-full max-w-6xl mx-auto p-6 md:p-10 xl:p-12",
                div { class: "mb-10 border-b border-slate-200 pb-7",
                    div { class: "flex flex-col gap-4 md:flex-row md:items-start md:justify-between",
                        div {
                            h2 { class: "text-4xl font-extrabold tracking-tight text-slate-900 mb-3", "Settings" }
                            p { class: "text-lg text-slate-600 max-w-2xl", "Tailor your AI experience. Manage local models, agent constraints, and application preferences." }
                        }
                        button {
                            class: "inline-flex items-center gap-2 self-start rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-semibold text-slate-700 shadow-sm hover:bg-slate-50 transition-all",
                            onclick: move |_| {
                                settings.restart();
                                models.restart();
                                runtime_health.restart();
                            },
                            MaterialIcon { name: "refresh", size: 18 }
                            "Refresh Health"
                        }
                    }
                }

                div { class: "space-y-9",
                    HealthSummary { health: health.clone(), default_chat_model: default_chat_model.clone(), title_model: title_model.clone() }
                    HealthDetails { health: health.clone() }
                    IntelligenceSection {
                        default_chat_model,
                        title_model,
                        max_iterations,
                        tool_timeout_ms,
                        model_timeout_ms,
                        available_models,
                        on_change
                    }
                    IntegritySection {
                        idle_lock_timeout_secs,
                        runtime_dir,
                        on_change
                    }
                    ConnectionSection {}
                }

                div { class: "mt-20 pt-8 border-t border-slate-200 text-center",
                    p { class: "text-sm font-bold text-slate-400 uppercase tracking-widest", "Overseer v{release_version} - Local runtime command center" }
                }
            }
        }
    }
}
