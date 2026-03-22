use crate::db::Planner;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn PlannerHeroCard(
    mut selected_planner: Signal<Option<Planner>>,
    models: Signal<Vec<String>>,
) -> Element {
    let planner = if let Some(p) = selected_planner() {
        p
    } else {
        return rsx! {};
    };

    rsx! {
        div { class: "bg-white p-8 rounded-2xl border border-slate-200 shadow-soft space-y-6 animate-in fade-in slide-in-from-top-4 duration-500",
            // Header: Icon + Primary Info
            div { class: "flex items-start justify-between",
                div { class: "flex items-center gap-6",
                    // Large Animated Icon
                    div { class: "w-16 h-16 bg-blue-50 rounded-2xl flex items-center justify-center border border-blue-100 shadow-sm group transition-all duration-500 hover:bg-brand-blue hover:shadow-blue-200 hover:shadow-xl",
                        MaterialIcon {
                            name: "architecture",
                            color: MaterialIconColor::Custom("var(--color-brand-blue)".into()),
                            size: 32,
                        }
                    }
                    // Name & Description Inputs
                    div { class: "space-y-1 flex-1 min-w-[300px]",
                        label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Planner Identity" }
                        input {
                            class: "text-2xl font-black text-slate-900 leading-tight bg-transparent border-none focus:ring-2 focus:ring-brand-blue/10 rounded-lg p-1 -ml-1 w-full transition-all placeholder-slate-200",
                            value: "{planner.name}",
                            placeholder: "Planner Name",
                            oninput: move |e| {
                                if let Some(mut p) = selected_planner() {
                                    p.name = e.value();
                                    selected_planner.set(Some(p));
                                }
                            }
                        }
                        textarea {
                            class: "text-slate-500 w-full bg-transparent border-none focus:ring-2 focus:ring-brand-blue/10 rounded-lg p-1 -ml-1 text-sm resize-none transition-all placeholder-slate-200 h-10 custom-scrollbar",
                            value: planner.description.clone().unwrap_or_default(),
                            placeholder: "Define the high-level orchestration strategy for this domain...",
                            oninput: move |e| {
                                if let Some(mut p) = selected_planner() {
                                    p.description = Some(e.value());
                                    selected_planner.set(Some(p));
                                }
                            }
                        }
                    }
                }

                // Status & Version Badges
                div { class: "flex flex-col items-end gap-3",
                    div { class: "flex items-center gap-2",
                        span { class: "px-2 py-0.5 bg-slate-100 text-slate-500 rounded text-[9px] font-black uppercase tracking-widest border border-slate-200",
                            "v{planner.version.clone().unwrap_or_else(|| \"1.0.0\".into())}"
                        }
                        span { class: "px-3 py-1 bg-blue-50 text-brand-blue rounded-full text-[10px] font-black uppercase tracking-widest border border-blue-100 animate-pulse",
                            "Ready"
                        }
                    }
                    div { class: "flex items-center gap-1.5",
                        div { class: "w-1.5 h-1.5 rounded-full bg-emerald-500 shadow-sm shadow-emerald-200" }
                        span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "State Synchronized" }
                    }
                }
            }

            // Configuration Grid
            div { class: "grid grid-cols-1 md:grid-cols-3 gap-8 pt-6 border-t border-slate-50",
                // Domain
                div { class: "space-y-2",
                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Execution Domain" }
                    div { class: "relative group",
                        input {
                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-700 focus:ring-2 focus:ring-brand-blue/10 focus:border-brand-blue transition-all outline-none",
                            value: "{planner.domain}",
                            placeholder: "e.g. general, web, code",
                            oninput: move |e| {
                                if let Some(mut p) = selected_planner() {
                                    p.domain = e.value();
                                    selected_planner.set(Some(p));
                                }
                            }
                        }
                        div { class: "absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-slate-300 group-focus-within:text-brand-blue transition-colors",
                            MaterialIcon { name: "public", size: 16 }
                        }
                    }
                }

                // LLM Model
                div { class: "space-y-2",
                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Primary Intelligence" }
                    div { class: "relative group",
                        select {
                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-700 appearance-none focus:ring-2 focus:ring-brand-blue/10 focus:border-brand-blue transition-all cursor-pointer outline-none",
                            value: planner.model.clone().unwrap_or_default(),
                            onchange: move |e| {
                                if let Some(mut p) = selected_planner() {
                                    p.model = Some(e.value());
                                    selected_planner.set(Some(p));
                                }
                            },
                            for model in models() {
                                option { value: "{model}", "{model}" }
                            }
                        }
                        div { class: "absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-slate-300 group-focus-within:text-brand-blue transition-colors",
                            MaterialIcon { name: "unfold_more", size: 18 }
                        }
                    }
                }

                // Version Control
                div { class: "space-y-2",
                    label { class: "block text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Version identifier" }
                    div { class: "relative group",
                        input {
                            class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-700 focus:ring-2 focus:ring-brand-blue/10 focus:border-brand-blue transition-all outline-none",
                            value: planner.version.clone().unwrap_or_else(|| "1.0.0".into()),
                            placeholder: "1.0.0",
                            oninput: move |e| {
                                if let Some(mut p) = selected_planner() {
                                    p.version = Some(e.value());
                                    selected_planner.set(Some(p));
                                }
                            }
                        }
                        div { class: "absolute right-4 top-1/2 -translate-y-1/2 pointer-events-none text-slate-300 group-focus-within:text-brand-blue transition-colors",
                            MaterialIcon { name: "history", size: 16 }
                        }
                    }
                }
            }
        }
    }
}
