use super::hero_card::PlannerHeroCard;
use crate::backend::{get_available_models, update_planner};
use crate::db::Planner;
use crate::state::use_global_state;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[derive(Clone, PartialEq)]
enum Tab {
    Preamble,
    Code,
}

impl Tab {
    fn label(&self) -> &'static str {
        match self {
            Tab::Preamble => "Strategy Preamble",
            Tab::Code => "Workflow Logic",
        }
    }
    fn icon(&self) -> &'static str {
        match self {
            Tab::Preamble => "description",
            Tab::Code => "code",
        }
    }
}

#[component]
pub fn PlannerDetails(
    mut selected_planner: Signal<Option<Planner>>,
    mut planners: Resource<Vec<Planner>>,
) -> Element {
    let state = use_global_state();

    // ── Available models ───────────────────────────────────────────────────
    let mut models = use_signal(|| vec!["loading...".to_string()]);
    let _ = use_resource(move || async move {
        let fetched: Vec<String> = get_available_models().await;
        if !fetched.is_empty() {
            models.set(fetched);
        }
    });

    // ── Local editing signals (synced when selection changes) ──────────────
    let p = selected_planner();
    let mut local_instructions = use_signal(|| {
        p.as_ref()
            .and_then(|sup| sup.instructions.clone())
            .unwrap_or_default()
    });
    let mut local_code = use_signal(|| {
        p.as_ref()
            .and_then(|sup| sup.code.clone())
            .unwrap_or_default()
    });

    use_effect(move || {
        if let Some(sup) = selected_planner() {
            local_instructions.set(sup.instructions.clone().unwrap_or_default());
            local_code.set(sup.code.clone().unwrap_or_default());
        }
    });

    // ── Active tab ─────────────────────────────────────────────────────────
    let mut active_tab = use_signal(|| Tab::Preamble);

    rsx! {
        div { class: "w-full bg-slate-50 overflow-y-auto p-8 custom-scrollbar h-full",
            if let Some(planner) = selected_planner() {
                div { class: "max-w-4xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500",

                    // ── Hero card ──────────────────────────────────────
                    PlannerHeroCard {
                        selected_planner,
                        models,
                    }

                    // ── Tab bar ────────────────────────────────────────
                    div { class: "flex gap-1 bg-white border border-slate-200 rounded-xl p-1 shadow-soft",
                        for tab in [Tab::Preamble, Tab::Code] {
                            {
                                let is_active = active_tab() == tab;
                                let tab_clone = tab.clone();
                                rsx! {
                                    button {
                                        key: "{tab.label()}",
                                        class: format!(
                                            "flex-1 flex items-center justify-center gap-2 px-4 py-2 rounded-lg text-xs font-bold transition-all {}",
                                            if is_active { "bg-brand-blue text-white shadow-md shadow-blue-100" }
                                            else { "text-slate-400 hover:text-slate-600 hover:bg-slate-50" }
                                        ),
                                        onclick: move |_| active_tab.set(tab_clone.clone()),
                                        MaterialIcon {
                                            name: tab.icon(),
                                            size: 16,
                                            color: if is_active { MaterialIconColor::Light } else { MaterialIconColor::Dark }
                                        }
                                        span { class: "uppercase tracking-wider", "{tab.label()}" }
                                    }
                                }
                            }
                        }
                    }

                    // ── Tab content ────────────────────────────────────
                    div { class: "min-h-[400px]",
                        match active_tab() {
                            Tab::Preamble => rsx! {
                                div { class: "space-y-4 animate-in fade-in duration-300",
                                    label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "System Strategy instructions" }
                                    div { class: "bg-white border border-slate-200 rounded-2xl overflow-hidden shadow-soft",
                                        textarea {
                                            class: "w-full min-h-[400px] p-6 text-sm leading-relaxed text-slate-700 focus:ring-2 focus:ring-blue-100 transition-all outline-none resize-y",
                                            value: "{local_instructions}",
                                            oninput: move |e| local_instructions.set(e.value()),
                                            spellcheck: false,
                                            placeholder: "Define the planning logic and strategy guidelines..."
                                        }
                                    }
                                }
                            },

                            Tab::Code => rsx! {
                                div { class: "space-y-4 animate-in fade-in duration-300",
                                    label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Workflow Logic (Modular Script)" }
                                    div { class: "bg-slate-900 rounded-2xl overflow-hidden border border-slate-800 shadow-xl",
                                        textarea {
                                            class: "w-full min-h-[400px] p-6 bg-transparent text-slate-300 font-mono text-xs leading-relaxed outline-none resize-none",
                                            value: "{local_code}",
                                            oninput: move |e| local_code.set(e.value()),
                                            spellcheck: false,
                                            placeholder: "// Custom workflow orchestration logic..."
                                        }
                                    }
                                }
                            },
                        }
                    }

                    // ── Save footer ────────────────────────────────────
                    div { class: "px-8 py-6 bg-white border border-slate-200 rounded-2xl shadow-soft flex justify-between items-center",
                        div { class: "flex items-center gap-2 text-slate-400",
                            MaterialIcon {
                                name: "auto_awesome_motion",
                                size: 18,
                                color: MaterialIconColor::Custom("var(--color-brand-blue)".into())
                            }
                            span { class: "text-[10px] font-bold uppercase tracking-widest", "Dynamic Orchestration Active" }
                        }
                        div { class: "flex gap-4",
                            button {
                                class: "px-6 py-3 bg-slate-50 hover:bg-slate-100 text-slate-500 rounded-xl text-xs font-bold uppercase tracking-widest transition-all",
                                onclick: move |_| selected_planner.set(None),
                                "Close"
                            }
                            button {
                                class: "bg-brand-blue text-white px-8 py-3 rounded-xl text-xs font-bold hover:bg-blue-500 transition-all shadow-md shadow-blue-100 active:scale-95 flex items-center gap-2",
                                onclick: move |_| {
                                    if let Some(mut p) = selected_planner() {
                                        p.instructions = Some(local_instructions());
                                        p.code = Some(local_code());
                                        let state = state;
                                        spawn(async move {
                                            let db = state.read().db.clone();
                                            let _ = update_planner(&db, p).await;
                                            planners.restart();
                                        });
                                    }
                                },
                                MaterialIcon { name: "save", size: 16 }
                                " Save & Deploy Planner "
                            }
                        }
                    }
                }
            } else {
                div { class: "h-full flex flex-col items-center justify-center text-center space-y-6",
                    div { class: "w-24 h-24 rounded-full bg-white border border-slate-200 shadow-soft flex items-center justify-center text-5xl", "🏗️" }
                    div {
                        h2 { class: "text-2xl font-bold text-slate-900", "Planners Workshop" }
                        p { class: "text-slate-400 mt-2 max-w-xs", "Select a domain planner to configure its strategy and underlying code logic." }
                    }
                }
            }
        }
    }
}
