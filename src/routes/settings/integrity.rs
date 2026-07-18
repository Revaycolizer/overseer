use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn IntegritySection(
    idle_lock_timeout_secs: String,
    runtime_dir: String,
    on_change: EventHandler<()>,
) -> Element {
    let state = crate::state::use_global_state();
    let nav = use_navigator();

    let timeout_mins = idle_lock_timeout_secs
        .parse::<i64>()
        .unwrap_or(1800) / 60;

    rsx! {
        section {
            div { class: "flex items-center gap-3 mb-8",
                div { class: "p-2 bg-slate-200 rounded-lg text-slate-600",
                    MaterialIcon { name: "shield", size: 24 }
                }
                div {
                    h3 { class: "text-xl font-bold text-slate-900", "System Integrity" }
                    p { class: "text-sm text-slate-500", "Privacy settings and local environment control." }
                }
            }

            div { class: "grid gap-6 xl:grid-cols-2",
                div { class: "group flex items-center justify-between p-6 rounded-2xl bg-white/75 transition-all duration-200",
                    div {
                        p { class: "text-base font-bold text-slate-900 block mb-1", "Strict Offline Mode" }
                        p { class: "text-sm text-slate-500", "Force all processing to happen on your local machine." }
                    }
                    div { class: "relative inline-flex items-center cursor-pointer",
                        input { checked: true, class: "sr-only peer", r#type: "checkbox" }
                        div { class: "w-14 h-8 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[4px] after:left-[4px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-6 after:w-6 after:transition-all peer-checked:bg-blue-600" }
                    }
                }

                div { class: "group flex items-center justify-between p-6 rounded-2xl bg-white/75 transition-all duration-200",
                    div {
                        p { class: "text-base font-bold text-slate-900 block mb-1", "Voice Calibration" }
                        p { class: "text-sm text-slate-500", "Train Overseer to recognize your voice for wake-word activation." }
                    }
                    button {
                        class: "bg-blue-50 text-blue-600 px-4 py-2 rounded-xl text-sm font-bold hover:bg-blue-100 transition-all",
                        onclick: move |_| { nav.push(crate::Route::VoiceTraining {}); },
                        "Calibrate"
                    }
                }

                div { class: "group flex flex-col md:flex-row md:items-center justify-between gap-4 p-6 rounded-2xl bg-white/75 transition-all duration-200",
                    div { class: "md:pr-4",
                        label { class: "text-base font-bold text-slate-900 block mb-1", "Idle Lock Timeout" }
                        p { class: "text-sm text-slate-500", "Inactivity timeout before displaying the secure lock screen." }
                    }
                    div { class: "flex items-center gap-3 w-full md:w-72",
                        input {
                            r#type: "number",
                            class: "bg-white border-slate-200 rounded-xl text-sm w-full focus:ring-4 focus:ring-blue-500/10 focus:border-blue-500 py-3 px-4 font-medium text-slate-700",
                            value: "{timeout_mins}",
                            min: "1",
                            max: "480",
                            onchange: move |evt| {
                                let db = state.read().db.clone();
                                let on_change = on_change.clone();
                                spawn(async move {
                                    let mins = evt.value().parse::<i64>().unwrap_or(30).max(1);
                                    let secs = mins * 60;
                                    let _ = crate::backend::set_setting(&db, "idle_lock_timeout_secs", &secs.to_string()).await;
                                    on_change.call(());
                                });
                            }
                        }
                        span { class: "text-xs font-bold text-slate-400 uppercase tracking-widest", "mins" }
                    }
                }

                div { class: "p-6 rounded-2xl bg-white/75",
                    div { class: "mb-4",
                        p { class: "text-base font-bold text-slate-900 mb-1", "Runtime Diagnostics" }
                        p { class: "text-sm text-slate-500", "Current local runtime paths used for native tool execution and staging." }
                    }
                    div { class: "grid gap-3 text-sm sm:grid-cols-2",
                        div {
                            span { class: "block text-xs font-bold text-slate-400 uppercase tracking-widest mb-1", "Core Runtime" }
                            code { class: "block rounded-xl bg-slate-900/[0.035] px-3.5 py-2.5 text-xs font-medium tracking-[0.01em] text-slate-700 break-all font-mono", "rust-native tool execution" }
                        }
                        div {
                            span { class: "block text-xs font-bold text-slate-400 uppercase tracking-widest mb-1", "Runtime Directory" }
                            code { class: "block rounded-xl bg-slate-900/[0.035] px-3.5 py-2.5 text-xs font-medium tracking-[0.01em] text-slate-700 break-all font-mono", "{runtime_dir}" }
                        }
                    }
                }
            }
        }
    }
}
