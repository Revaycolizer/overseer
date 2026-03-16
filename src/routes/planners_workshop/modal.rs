use dioxus::prelude::*;
use crate::db::Planner;

#[component]
pub fn PlannerModal(
    mut show_modal: Signal<bool>,
    editing_planner: Signal<Option<Planner>>,
    mut name_input: Signal<String>,
    mut description_input: Signal<String>,
    mut domain_input: Signal<String>,
    handle_save: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-[#0F172A]/40 backdrop-blur-sm animate-in fade-in duration-200",
            div { class: "w-full max-w-md bg-white border border-[#E2E8F0] rounded-2xl shadow-2xl p-6 space-y-6 animate-in zoom-in-95 duration-200",
                div {
                    h2 { class: "text-lg font-bold text-[#0F172A]", 
                        if editing_planner().is_some() { "Edit Planner" } else { "Create New Planner" }
                    }
                    p { class: "text-sm text-slate-500", "Define a new hierarchical planner for a specific task domain." }
                }
                
                div { class: "space-y-4",
                    div { class: "space-y-1.5",
                        label { class: "text-xs font-bold text-slate-400 mt-1 inline-block border-slate-400 uppercase tracking-widest", "Planner Name" }
                        input {
                            class: "w-full bg-slate-50 border border-[#E2E8F0] rounded-xl px-4 py-3 text-sm focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6] outline-none transition-all placeholder-slate-300",
                            placeholder: "e.g., Office Planner",
                            value: "{name_input}",
                            oninput: move |e| name_input.set(e.value()),
                        }
                    }

                    div { class: "space-y-1.5",
                        label { class: "text-xs font-bold text-slate-400 mt-1 inline-block border-slate-400 uppercase tracking-widest", "Description" }
                        input {
                            class: "w-full bg-slate-50 border border-[#E2E8F0] rounded-xl px-4 py-3 text-sm focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6] outline-none transition-all placeholder-slate-300",
                            placeholder: "e.g., Handles step-by-step planning for office apps.",
                            value: "{description_input}",
                            oninput: move |e| description_input.set(e.value()),
                        }
                    }

                    div { class: "space-y-1.5",
                        label { class: "text-xs font-bold text-slate-400 mt-1 inline-block border-slate-400 uppercase tracking-widest", "Domain Identifier" }
                        input {
                            class: "w-full bg-slate-50 border border-[#E2E8F0] rounded-xl px-4 py-3 text-sm font-mono focus:ring-2 focus:ring-[#3B82F6]/20 focus:border-[#3B82F6] outline-none transition-all placeholder-slate-300",
                            placeholder: "e.g., office_planner",
                            value: "{domain_input}",
                            oninput: move |e| domain_input.set(e.value()),
                        }
                    }
                }

                div { class: "flex gap-3 pt-2",
                    button { 
                        class: "flex-1 px-4 py-3 rounded-xl border border-[#E2E8F0] text-sm font-bold text-slate-600 hover:bg-slate-50 transition-all",
                        onclick: move |_| show_modal.set(false),
                        "Cancel"
                    }
                    button { 
                        class: "flex-1 px-4 py-3 rounded-xl bg-slate-900 text-white text-sm font-bold hover:bg-slate-800 transition-all shadow-lg active:scale-[0.98]",
                        onclick: move |_| handle_save.call(()),
                        if editing_planner().is_some() { "Update Planner" } else { "Create Planner" }
                    }
                }
            }
        }
    }
}
