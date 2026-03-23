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

    let mut models = use_signal(|| vec!["loading...".to_string()]);
    let _ = use_resource(move || async move {
        let fetched = get_available_models().await;
        if !fetched.is_empty() {
            models.set(fetched);
        }
    });

    let planner = selected_planner();
    let mut local_instructions = use_signal(|| {
        planner
            .as_ref()
            .and_then(|item| item.instructions.clone())
            .unwrap_or_default()
    });
    let mut local_code = use_signal(|| {
        planner
            .as_ref()
            .and_then(|item| item.code.clone())
            .unwrap_or_default()
    });
    let mut save_feedback = use_signal(|| "Planner strategy is synced.".to_string());
    let mut save_state = use_signal(|| "synced".to_string());
    let mut active_tab = use_signal(|| Tab::Preamble);

    use_effect(move || {
        if let Some(item) = selected_planner() {
            local_instructions.set(item.instructions.clone().unwrap_or_default());
            local_code.set(item.code.clone().unwrap_or_default());
            save_feedback.set("Planner strategy is synced.".to_string());
            save_state.set("synced".to_string());
        }
    });

    rsx! {
        div { class: "w-full bg-slate-50 overflow-y-auto p-8 custom-scrollbar h-full",
            if let Some(planner_final) = selected_planner() {
                {
                    let instruction_text = local_instructions();
                    let code_text = local_code();
                    let instruction_lines = instruction_text.lines().count().max(1);
                    let code_lines = code_text.lines().count().max(1);
                    let strategy_tokens = instruction_text.split_whitespace().count();
                    let logic_blocks = code_text
                        .lines()
                        .filter(|line| {
                            let trimmed = line.trim_start();
                            trimmed.starts_with("fn ")
                                || trimmed.starts_with("if ")
                                || trimmed.starts_with("match ")
                                || trimmed.starts_with("for ")
                                || trimmed.starts_with("while ")
                        })
                        .count();
                    let planning_profile = if code_lines > 30 { "deeper" } else { "lighter" };
                    let is_dirty = planner_final.instructions.clone().unwrap_or_default() != instruction_text
                        || planner_final.code.clone().unwrap_or_default() != code_text;
                    let display_state = if is_dirty && save_state() == "synced" {
                        "draft".to_string()
                    } else {
                        save_state()
                    };
                    let display_feedback = if is_dirty && save_state() == "synced" {
                        "Draft planning changes are local until you deploy them.".to_string()
                    } else {
                        save_feedback()
                    };

                    rsx! {
                        div { class: "max-w-6xl mx-auto space-y-8 animate-in fade-in slide-in-from-bottom-4 duration-500",
                            PlannerHeroCard {
                                selected_planner,
                                models,
                                strategy_tokens,
                                logic_blocks,
                                workbench_status: display_feedback.clone(),
                                save_state: display_state.clone(),
                            }

                            div { class: "grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_320px] gap-8",
                                div { class: "space-y-8",
                                    div { class: "grid grid-cols-1 sm:grid-cols-2 xl:grid-cols-4 gap-4",
                                        DetailMetricCard {
                                            icon: "route",
                                            label: "Execution Domain",
                                            value: planner_final.domain.clone(),
                                            hint: "Primary routing lane"
                                        }
                                        DetailMetricCard {
                                            icon: "description",
                                            label: "Strategy Lines",
                                            value: instruction_lines.to_string(),
                                            hint: "Preamble depth"
                                        }
                                        DetailMetricCard {
                                            icon: "account_tree",
                                            label: "Logic Blocks",
                                            value: logic_blocks.to_string(),
                                            hint: "Code decision branches"
                                        }
                                        DetailMetricCard {
                                            icon: "smart_toy",
                                            label: "Runtime Model",
                                            value: planner_final.model.clone().unwrap_or_else(|| "default".to_string()),
                                            hint: "Primary planning model"
                                        }
                                    }

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
                                                            if is_active {
                                                                "bg-brand-blue text-white shadow-md shadow-blue-100"
                                                            } else {
                                                                "text-slate-400 hover:text-slate-600 hover:bg-slate-50"
                                                            }
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

                                    div { class: "min-h-[420px]",
                                        match active_tab() {
                                            Tab::Preamble => rsx! {
                                                div { class: "space-y-4 animate-in fade-in duration-300",
                                                    label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "System Strategy Instructions" }
                                                    div { class: "bg-white border border-slate-200 rounded-2xl overflow-hidden shadow-soft",
                                                        textarea {
                                                            class: "w-full min-h-[420px] p-6 text-sm leading-relaxed text-slate-700 focus:ring-2 focus:ring-blue-100 transition-all outline-none resize-y",
                                                            value: "{local_instructions}",
                                                            oninput: move |e| local_instructions.set(e.value()),
                                                            spellcheck: false,
                                                            placeholder: "Describe how this planner should classify requests, build steps, and escalate to supervisors..."
                                                        }
                                                    }
                                                }
                                            },
                                            Tab::Code => rsx! {
                                                div { class: "space-y-4 animate-in fade-in duration-300",
                                                    label { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Workflow Logic (Modular Script)" }
                                                    div { class: "bg-slate-900 rounded-2xl overflow-hidden border border-slate-800 shadow-xl",
                                                        textarea {
                                                            class: "w-full min-h-[420px] p-6 bg-transparent text-slate-300 font-mono text-xs leading-relaxed outline-none resize-none",
                                                            value: "{local_code}",
                                                            oninput: move |e| local_code.set(e.value()),
                                                            spellcheck: false,
                                                            placeholder: "// Define deterministic planning helpers, routing hints, or domain-specific orchestration logic..."
                                                        }
                                                    }
                                                }
                                            },
                                        }
                                    }

                                    div { class: "px-8 py-6 bg-white border border-slate-200 rounded-2xl shadow-soft flex justify-between items-center",
                                        div { class: "flex items-center gap-2 text-slate-400",
                                            MaterialIcon {
                                                name: "lan",
                                                size: 18,
                                                color: MaterialIconColor::Custom("var(--color-brand-blue)".into())
                                            }
                                            span { class: "text-[10px] font-bold uppercase tracking-widest", "{display_feedback}" }
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
                                                    if let Some(mut planner) = selected_planner() {
                                                        planner.instructions = Some(local_instructions());
                                                        planner.code = Some(local_code());
                                                        save_state.set("saving".to_string());
                                                        save_feedback.set("Deploying planner strategy...".to_string());
                                                        let state = state;
                                                        spawn(async move {
                                                            let db = state.read().db.clone();
                                                            let result = update_planner(&db, planner).await;
                                                            match result {
                                                                Ok(_) => {
                                                                    save_feedback.set("Planner strategy deployed successfully.".to_string());
                                                                    save_state.set("synced".to_string());
                                                                    planners.restart();
                                                                }
                                                                Err(_) => {
                                                                    save_feedback.set("Failed to deploy planner strategy.".to_string());
                                                                    save_state.set("error".to_string());
                                                                }
                                                            }
                                                        });
                                                    }
                                                },
                                                MaterialIcon { name: "save", size: 16 }
                                                "Save & Deploy Planner"
                                            }
                                        }
                                    }
                                }

                                div { class: "space-y-4",
                                    SidePanelCard {
                                        title: "Routing Readout",
                                        icon: "alt_route",
                                        body: "This planner owns the {planner_final.domain} lane and should keep routing deterministic before escalating to deeper orchestration."
                                    }
                                    SidePanelCard {
                                        title: "Workbench Status",
                                        icon: "insights",
                                        body: "{display_feedback}"
                                    }
                                    SidePanelCard {
                                        title: "Planning Shape",
                                        icon: "stacked_line_chart",
                                        body: "Current draft has {strategy_tokens} strategy tokens and {code_lines} lines of logic, giving this planner a {planning_profile} execution profile."
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "h-full flex flex-col items-center justify-center text-center space-y-6",
                    div { class: "w-24 h-24 rounded-full bg-white border border-slate-200 shadow-soft flex items-center justify-center text-5xl", "🏗️" }
                    div {
                        h2 { class: "text-2xl font-bold text-slate-900", "Planners Workshop" }
                        p { class: "text-slate-400 mt-2 max-w-xs", "Select a domain planner to configure its strategy, routing posture, and underlying orchestration logic." }
                    }
                }
            }
        }
    }
}

#[component]
fn DetailMetricCard(icon: String, label: String, value: String, hint: String) -> Element {
    rsx! {
        div { class: "bg-white border border-slate-200 rounded-2xl p-5 shadow-soft space-y-3",
            div { class: "flex items-center justify-between",
                span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "{label}" }
                MaterialIcon {
                    name: "{icon}",
                    size: 16,
                    color: MaterialIconColor::Custom("var(--color-brand-blue)".into())
                }
            }
            div { class: "text-lg font-black text-slate-900 break-words", "{value}" }
            p { class: "text-xs text-slate-400 leading-relaxed", "{hint}" }
        }
    }
}

#[component]
fn SidePanelCard(title: String, icon: String, body: String) -> Element {
    rsx! {
        div { class: "bg-white border border-slate-200 rounded-2xl p-5 shadow-soft space-y-3",
            div { class: "flex items-center gap-2",
                MaterialIcon {
                    name: "{icon}",
                    size: 16,
                    color: MaterialIconColor::Custom("var(--color-brand-blue)".into())
                }
                h3 { class: "text-sm font-black text-slate-900", "{title}" }
            }
            p { class: "text-sm text-slate-500 leading-relaxed", "{body}" }
        }
    }
}
