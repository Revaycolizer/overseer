use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::db::Supervisor;

#[component]
pub fn SupervisorHeroCard(
    mut selected_supervisor: Signal<Option<Supervisor>>,
    models: Signal<Vec<String>>,
) -> Element {
    let Some(supervisor) = selected_supervisor() else {
        return rsx! {};
    };

    rsx! {
        div { class: "bg-white p-8 rounded-2xl border border-[#E2E8F0] shadow-soft space-y-6",

            // ── Top row: icon + name/description + version/domain ──────
            div { class: "flex items-start justify-between",

                // Left: icon + editable name + description
                div { class: "flex items-center gap-6",
                    div { class: "w-16 h-16 bg-blue-50 rounded-2xl flex items-center justify-center border border-blue-100 shadow-sm",
                        MaterialIcon {
                            name: "account_tree",
                            color: MaterialIconColor::Custom("#3b82f6".into()),
                            size: 32
                        }
                    }
                    div { class: "space-y-1",
                        label {
                            class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                            "Supervisor Identity"
                        }
                        input {
                            class: "text-2xl font-extrabold text-[#0F172A] leading-tight bg-transparent border-none focus:ring-1 focus:ring-blue-100 rounded p-1 -ml-1 w-full",
                            value: "{supervisor.name}",
                            oninput: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    sv.name = e.value();
                                    selected_supervisor.set(Some(sv.clone()));
                                }
                            }
                        }
                        input {
                            class: "text-slate-500 w-full bg-transparent border-none focus:ring-1 focus:ring-blue-100 rounded p-1 -ml-1",
                            value: supervisor.description.clone().unwrap_or_default(),
                            placeholder: "Add a description...",
                            oninput: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    sv.description = Some(e.value());
                                    selected_supervisor.set(Some(sv.clone()));
                                }
                            }
                        }
                    }
                }

                // Right: version badge + domain input
                div { class: "flex flex-col items-end gap-2",
                    div { class: "flex items-center gap-2",
                        span {
                            class: "px-2 py-0.5 bg-slate-100 text-slate-500 rounded text-[9px] font-bold tracking-widest",
                            "v{supervisor.version.clone().unwrap_or_else(|| \"1.0.0\".into())}"
                        }
                        span {
                            class: "px-3 py-1 bg-blue-50 text-[#3B82F6] rounded-full text-[10px] font-bold uppercase tracking-widest",
                            "Active"
                        }
                    }
                    div {
                        label {
                            class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest text-right mb-1",
                            "Domain"
                        }
                        input {
                            class: "text-[10px] font-bold text-slate-500 bg-slate-100 rounded px-2 py-0.5 border-none focus:ring-0 text-right w-24",
                            value: "{supervisor.domain}",
                            oninput: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    sv.domain = e.value();
                                    selected_supervisor.set(Some(sv.clone()));
                                }
                            }
                        }
                    }
                }
            }

            // ── Bottom row: model dropdown + version input ─────────────
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-8 pt-4 border-t border-slate-50",

                // Model Assignment
                div {
                    label {
                        class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3",
                        "Model Assignment"
                    }
                    div { class: "relative",
                        select {
                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700 appearance-none focus:ring-2 focus:ring-blue-100 focus:border-blue-400 transition-all cursor-pointer",
                            value: supervisor.model.clone().unwrap_or_default(),
                            onchange: {
                                let mut sv = supervisor.clone();
                                move |e| {
                                    sv.model = Some(e.value());
                                    selected_supervisor.set(Some(sv.clone()));
                                }
                            },
                            for model in models() {
                                option { value: "{model}", "{model}" }
                            }
                        }
                        div { class: "absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-slate-400",
                            MaterialIcon { name: "unfold_more", size: 18 }
                        }
                    }
                }

                // Version
                div {
                    label {
                        class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3",
                        "Version"
                    }
                    input {
                        class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700 focus:ring-2 focus:ring-blue-100 focus:border-blue-400 transition-all",
                        value: supervisor.version.clone().unwrap_or_else(|| "1.0.0".into()),
                        oninput: {
                            let mut sv = supervisor.clone();
                            move |e| {
                                sv.version = Some(e.value());
                                selected_supervisor.set(Some(sv.clone()));
                            }
                        }
                    }
                }
            }
        }
    }
}
