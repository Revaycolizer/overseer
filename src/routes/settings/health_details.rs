use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use crate::routes::settings::components::HealthCard;

#[component]
pub fn HealthDetails(health: Option<crate::backend::RuntimeHealth>) -> Element {
    rsx! {
        section {
            div { class: "flex items-center gap-3 mb-8",
                div { class: "p-2 bg-emerald-100 rounded-lg text-emerald-600",
                    MaterialIcon { name: "monitor_heart", size: 24 }
                }
                div {
                    h3 { class: "text-xl font-bold text-slate-900", "Runtime Health" }
                    p { class: "text-sm text-slate-500", "Verify that Ollama, your selected model, and local dependencies are ready." }
                }
            }

            if let Some(h) = health {
                div { class: "grid gap-4 md:grid-cols-2",
                    HealthCard {
                        title: "Ollama".to_string(),
                        status: if h.ollama_reachable { "reachable".to_string() } else { "offline".to_string() },
                        good: h.ollama_reachable,
                        body: if h.ollama_reachable {
                            format!("Endpoint: {}. {} model(s) discovered.", h.ollama_endpoint, h.available_models.len())
                        } else {
                            format!("Endpoint: {}. Start Ollama, then refresh this panel.", h.ollama_endpoint)
                        },
                        code: Some(h.ollama_endpoint.clone()),
                        icon: "hub".to_string()
                    }

                    HealthCard {
                        title: "Selected Runtime Model".to_string(),
                        status: if h.runtime_model_available { "ready".to_string() } else { "missing".to_string() },
                        good: h.runtime_model_available,
                        body: "This is the default model used when Overseer routes chat and orchestration work.".to_string(),
                        code: Some(h.selected_runtime_model.clone()),
                        icon: "smart_toy".to_string()
                    }

                    HealthCard {
                        title: "Native Tool Runtime".to_string(),
                        status: if h.native_runtime { "active".to_string() } else { "offline".to_string() },
                        good: h.native_runtime,
                        body: "Core Overseer tools now execute in the native runtime instead of depending on Python.".to_string(),
                        code: Some("rust-native core tools".to_string()),
                        icon: "terminal".to_string()
                    }

                    HealthCard {
                        title: "Python Extensions".to_string(),
                        status: if h.python_command.is_some() { "available".to_string() } else { "optional".to_string() },
                        good: true,
                        body: if h.python_command.is_some() {
                            "Advanced and legacy extension tools can still run through Python when needed.".to_string()
                        } else {
                            "Python is not required for core use, but some advanced extension tools will stay unavailable until it is installed.".to_string()
                        },
                        code: Some(h.python_command.clone().unwrap_or_else(|| "python (optional, not found)".to_string())),
                        icon: "integration_instructions".to_string()
                    }

                    HealthCard {
                        title: "protoc".to_string(),
                        status: if h.protoc_path.is_some() { "found".to_string() } else { "missing".to_string() },
                        good: h.protoc_path.is_some(),
                        body: "Build generation and protobuf-dependent flows look here first.".to_string(),
                        code: Some(h.protoc_path.clone().unwrap_or_else(|| "protoc (not found)".to_string())),
                        icon: "deployed_code".to_string()
                    }
                }

                div { class: "mt-4 grid gap-4 lg:grid-cols-2",
                    HealthCard {
                        title: "Utility Model".to_string(),
                        status: if h.title_model_available { "ready".to_string() } else { "missing".to_string() },
                        good: h.title_model_available,
                        body: "This lighter model handles conversation titles and small summary tasks.".to_string(),
                        code: Some(h.title_model.clone()),
                        icon: "auto_awesome".to_string()
                    }
                    HealthCard {
                        title: "Runtime Directory".to_string(),
                        status: "active".to_string(),
                        good: true,
                        body: "Overseer stages helper scripts and runtime files in this local directory.".to_string(),
                        code: Some(h.runtime_dir.clone()),
                        icon: "folder_managed".to_string()
                    }
                }
            } else {
                div { class: "p-6 rounded-2xl bg-white/80 text-sm text-slate-500 flex items-center gap-3",
                    div { class: "size-4 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" }
                    "Checking local runtime health..."
                }
            }
        }
    }
}
