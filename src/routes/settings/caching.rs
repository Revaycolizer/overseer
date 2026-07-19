use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

/// Settings section for cache configuration.
///
/// Overseer uses an in-memory LRU cache by default. For multi-node or
/// high-traffic deployments, a Redis backend can be configured.
#[component]
pub fn CachingSection() -> Element {
    let state = crate::state::use_global_state();
    let db = state.read().db.clone();

    let mut url_input = use_signal(|| String::new());
    let mut saved_msg = use_signal(|| String::new());

    let db_for_resource = db.clone();
    // Load redis_url setting on first render
    let stored_url = use_resource(move || {
        let db = db_for_resource.clone();
        async move {
            crate::backend::get_setting(&db, "redis_url").await
        }
    });

    // Initialise input field when stored_url loads
    let stored_val = stored_url.read().clone();
    if let Some(ref url) = stored_val {
        if url_input.read().is_empty() {
            url_input.set(url.clone());
        }
    }

    let use_redis = stored_val.as_ref().map_or(false, |s| !s.is_empty());
    let db_clone = db.clone();

    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-white p-6 space-y-5",
            div { class: "flex items-center gap-3",
                MaterialIcon { name: "storage", size: 22 }
                h3 { class: "text-lg font-bold text-slate-900", "Caching" }
            }

            p { class: "text-sm text-slate-500 -mt-2",
                "Overseer caches messages, workflows, and browser state in memory. \
                 Optionally connect Redis for a shared, persistent cache layer."
            }

            if !saved_msg.read().is_empty() {
                div { class: "rounded-lg bg-emerald-50 border border-emerald-200 px-4 py-3 text-sm text-emerald-700",
                    "{saved_msg}"
                }
            }

            // Current cache status
            div { class: "rounded-xl bg-slate-50 p-4 space-y-2",
                div { class: "flex items-center justify-between",
                    span { class: "text-sm font-medium text-slate-700", "Cache Backend" }
                    if use_redis {
                        span { class: "inline-flex items-center gap-1 rounded-full bg-emerald-100 px-2.5 py-0.5 text-xs font-semibold text-emerald-700",
                            MaterialIcon { name: "hub", size: 12 }
                            "Redis"
                        }
                    } else {
                        span { class: "inline-flex items-center gap-1 rounded-full bg-slate-200 px-2.5 py-0.5 text-xs font-semibold text-slate-600",
                            MaterialIcon { name: "memory", size: 12 }
                            "In-Memory LRU"
                        }
                    }
                }
                div { class: "flex items-center justify-between",
                    span { class: "text-sm font-medium text-slate-700", "Message Cache" }
                    span { class: "text-xs text-slate-500", "32 entries (LRU)" }
                }
                div { class: "flex items-center justify-between",
                    span { class: "text-sm font-medium text-slate-700", "Workflow Cache" }
                    span { class: "text-xs text-slate-500", "64 entries (LRU)" }
                }
                div { class: "flex items-center justify-between",
                    span { class: "text-sm font-medium text-slate-700", "Browser State Cache" }
                    span { class: "text-xs text-slate-500", "16 entries (LRU)" }
                }
            }

            // Redis configuration
            div { class: "space-y-3",
                h4 { class: "text-sm font-semibold text-slate-700", "Redis Connection" }
                p { class: "text-xs text-slate-500",
                    "Leave empty to use the default in-memory cache. \
                     Set a Redis URL to enable a shared cache backend."
                }
                div { class: "flex gap-2",
                    input {
                        class: "flex-1 rounded-lg border border-slate-300 bg-white px-3 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-slate-400",
                        placeholder: "redis://127.0.0.1:6379",
                        value: "{url_input}",
                        oninput: move |e| url_input.set(e.value()),
                    }
                    button {
                        class: "rounded-lg bg-slate-900 px-3 py-2 text-xs font-semibold text-white hover:bg-slate-700 transition-all",
                        onclick: move |_| {
                            let db = db_clone.clone();
                            let val = url_input.read().clone();
                            saved_msg.set(String::new());
                            spawn(async move {
                                match crate::backend::set_setting(&db, "redis_url", &val).await {
                                    Ok(_) => {
                                        saved_msg.set("Redis URL saved. Restart Overseer to apply the new cache backend.".to_string());
                                    }
                                    Err(e) => {
                                        saved_msg.set(format!("Failed to save: {}", e));
                                    }
                                }
                            });
                        },
                        "Save"
                    }
                }
            }
        }
    }
}
