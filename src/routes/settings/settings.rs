use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};


#[component]
pub fn Settings() -> Element {
    let state = crate::state::use_global_state();
    let db = state.read().db.clone();
    
    let mut settings = use_resource(move || {
        let db = db.clone();
        async move {
            crate::backend::get_all_settings(&db).await
        }
    });

    let mut models = use_resource(move || async move {
        crate::backend::get_available_models().await
    });

    let current_settings = settings.read().cloned().unwrap_or_default();
    let available_models = models.read().cloned().unwrap_or_default();

    let title_model = current_settings.get("title_generator_model")
        .cloned()
        .unwrap_or_else(|| "qwen2.5:7b-instruct".to_string());
    
    let default_chat_model = current_settings.get("default_chat_model")
        .cloned()
        .unwrap_or_else(|| "llama3:latest".to_string());

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0 bg-background-light overflow-y-auto custom-scrollbar",
            main { class: "flex-1 p-8 max-w-5xl mx-auto w-full",
                div { class: "mb-10",
                    h2 { class: "text-3xl font-extrabold tracking-tight text-slate-900 mb-2",
                        "Settings"
                    }
                    p { class: "text-base text-slate-500",
                        "Configure your local AI environment and application preferences."
                    }
                }
                
                div { class: "settings-section",
                    div { class: "flex items-center gap-3 mb-6",
                        MaterialIcon { name: "psychology", color: MaterialIconColor::Custom("text-slate-400".to_string()), size: 24 }
                        h3 { class: "text-lg font-bold text-slate-900", "AI Engine" }
                    }
                    div { class: "space-y-6",
                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-sm font-semibold text-slate-900",
                                    "Default Chat Model"
                                }
                                p { class: "text-xs text-slate-500",
                                    "Select which local model to use for primary chat."
                                }
                            }
                            select { 
                                class: "bg-white border-slate-200 rounded-lg text-sm w-64 focus:ring-1 focus:ring-brand-blue/20 focus:border-brand-blue py-2 px-3 shadow-sm cursor-pointer",
                                value: "{default_chat_model}",
                                onchange: move |evt| {
                                    let db = state.read().db.clone();
                                    let mut settings = settings.clone();
                                    spawn(async move {
                                        let _ = crate::backend::set_setting(&db, "default_chat_model", &evt.value()).await;
                                        settings.restart();
                                    });
                                },
                                for model in &available_models {
                                    option { value: "{model}", selected: *model == default_chat_model, "{model}" }
                                }
                            }
                        }

                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-sm font-semibold text-slate-900",
                                    "Title Generator Model"
                                }
                                p { class: "text-xs text-slate-500",
                                    "Select which model to use for generating conversation titles."
                                }
                            }
                            select { 
                                class: "bg-white border-slate-200 rounded-lg text-sm w-64 focus:ring-1 focus:ring-brand-blue/20 focus:border-brand-blue py-2 px-3 shadow-sm cursor-pointer",
                                value: "{title_model}",
                                onchange: move |evt| {
                                    let db = state.read().db.clone();
                                    let mut settings = settings.clone();
                                    spawn(async move {
                                        let _ = crate::backend::set_setting(&db, "title_generator_model", &evt.value()).await;
                                        settings.restart();
                                    });
                                },
                                for model in &available_models {
                                    option { value: "{model}", selected: *model == title_model, "{model}" }
                                }
                            }
                        }
                    }
                }

                div { class: "settings-section",
                    div { class: "flex items-center gap-3 mb-6",
                        MaterialIcon { name: "shield", color: MaterialIconColor::Custom("text-slate-400".to_string()), size: 24 }
                        h3 { class: "text-lg font-bold text-slate-900", "Privacy & Security" }
                    }
                    div { class: "space-y-6",
                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-sm font-semibold text-slate-900",
                                    "Local Only Mode"
                                }
                                p { class: "text-xs text-slate-500",
                                    "Prevent any external API calls for processing."
                                }
                            }
                            div { class: "relative inline-flex items-center cursor-pointer",
                                input {
                                    checked: true,
                                    class: "sr-only peer",
                                    r#type: "checkbox",
                                }
                                div { class: "w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-brand-blue" }
                            }
                        }
                    }
                }
            }
        }
    }
}

