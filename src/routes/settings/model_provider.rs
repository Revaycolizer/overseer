use dioxus::prelude::*;

/// Choose the LLM backend (local Ollama or a cloud provider) and set API keys.
/// Cloud providers give reliable replies and (soon) native tool-calling; Ollama
/// stays the offline default.
#[component]
pub fn ModelProviderSection() -> Element {
    let db = crate::state::use_global_state().read().db.clone();

    let db_load = db.clone();
    let loaded = use_resource(move || {
        let db = db_load.clone();
        async move {
            (
                crate::backend::get_setting_or(&db, "llm_provider", "ollama").await,
                crate::backend::get_setting(&db, "openai_api_key").await,
                crate::backend::get_setting_or(&db, "openai_model", "gpt-4o-mini").await,
                crate::backend::get_setting(&db, "anthropic_api_key").await,
                crate::backend::get_setting_or(&db, "anthropic_model", "claude-3-5-sonnet-latest").await,
            )
        }
    });
    let (provider, openai_key, openai_model, anthropic_key, anthropic_model) =
        loaded.read().clone().unwrap_or_else(|| {
            (
                "ollama".into(),
                String::new(),
                "gpt-4o-mini".into(),
                String::new(),
                "claude-3-5-sonnet-latest".into(),
            )
        });

    // Save helper: each call clones what it needs so it can be used from every
    // handler (closures are not Clone, so we build a fresh one per input).
    macro_rules! on_save {
        ($key:literal) => {{
            let db = db.clone();
            move |e: Event<FormData>| {
                let db = db.clone();
                let mut loaded = loaded;
                let value = e.value();
                spawn(async move {
                    let _ = crate::backend::set_setting(&db, $key, &value).await;
                    loaded.restart();
                });
            }
        }};
    }

    rsx! {
        section { class: "rounded-2xl border border-slate-200 bg-white p-6",
            h3 { class: "text-lg font-semibold text-slate-900", "Model Provider" }
            p { class: "text-sm text-slate-500 mt-1 mb-5", "Local Ollama by default. Add a cloud key for faster, more reliable replies." }

            div { class: "space-y-4 max-w-xl",
                div {
                    label { class: "block text-xs font-medium text-slate-500 mb-1.5", "Provider" }
                    select {
                        class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm bg-white",
                        value: "{provider}",
                        onchange: on_save!("llm_provider"),
                        option { value: "ollama", "Ollama (local)" }
                        option { value: "openai", "OpenAI" }
                        option { value: "anthropic", "Anthropic (Claude)" }
                    }
                }

                if provider == "openai" {
                    div {
                        label { class: "block text-xs font-medium text-slate-500 mb-1.5", "OpenAI API key" }
                        input {
                            r#type: "password",
                            class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm font-mono",
                            placeholder: "sk-…",
                            value: "{openai_key}",
                            onchange: on_save!("openai_api_key"),
                        }
                        label { class: "block text-xs font-medium text-slate-500 mt-3 mb-1.5", "Model" }
                        input {
                            class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm",
                            value: "{openai_model}",
                            onchange: on_save!("openai_model"),
                        }
                    }
                }

                if provider == "anthropic" {
                    div {
                        label { class: "block text-xs font-medium text-slate-500 mb-1.5", "Anthropic API key" }
                        input {
                            r#type: "password",
                            class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm font-mono",
                            placeholder: "sk-ant-…",
                            value: "{anthropic_key}",
                            onchange: on_save!("anthropic_api_key"),
                        }
                        label { class: "block text-xs font-medium text-slate-500 mt-3 mb-1.5", "Model" }
                        input {
                            class: "w-full border border-slate-200 rounded-lg px-3 py-2 text-sm",
                            value: "{anthropic_model}",
                            onchange: on_save!("anthropic_model"),
                        }
                    }
                }
            }
        }
    }
}
