use dioxus::prelude::*;
use crate::db::Supervisor;

#[component]
pub fn SupervisorModal(
    mut show_modal: Signal<bool>,
    editing_supervisor: Signal<Option<Supervisor>>,
    mut name_input: Signal<String>,
    mut description_input: Signal<String>,
    mut domain_input: Signal<String>,
    handle_save: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "fixed inset-0 z-50 flex items-center justify-center p-4 bg-[#0F172A]/40 backdrop-blur-sm animate-in fade-in duration-200",
            div { class: "w-full max-w-md bg-white border border-[#E2E8F0] rounded-2xl shadow-2xl p-6 space-y-6 animate-in zoom-in-95 duration-200",
                div {
                    h2 { class: "text-lg font-bold text-[#0F172A]", if editing_supervisor().is_some() { "Edit Supervisor" } else { "New Supervisor" } }
                    p { class: "text-slate-400 text-sm mt-1", "Define the domain and name for this orchestrator." }
                }
                div { class: "space-y-4",
                    div { class: "space-y-2",
                        label { class: "text-xs font-semibold text-slate-400 uppercase tracking-wider", "Display Name" }
                        input { 
                            class: "w-full bg-[#F8FAFC] border border-[#E2E8F0] rounded-lg p-3 text-[#0F172A] focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/30 focus:border-[#3B82F6]/50 transition-all placeholder-slate-300",
                            placeholder: "e.g. Research Supervisor",
                            value: "{name_input}",
                            oninput: move |e| name_input.set(e.value())
                        }
                    }
                    div { class: "space-y-2",
                        label { class: "text-xs font-semibold text-slate-400 uppercase tracking-wider", "Description" }
                        input { 
                            class: "w-full bg-[#F8FAFC] border border-[#E2E8F0] rounded-lg p-3 text-[#0F172A] focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/30 focus:border-[#3B82F6]/50 transition-all placeholder-slate-300",
                            placeholder: "e.g. Handles web tasks",
                            value: "{description_input}",
                            oninput: move |e| description_input.set(e.value())
                        }
                    }
                    div { class: "space-y-2",
                        label { class: "text-xs font-semibold text-slate-400 uppercase tracking-wider", "Execution Domain" }
                        input { 
                            class: "w-full bg-[#F8FAFC] border border-[#E2E8F0] rounded-lg p-3 text-[#0F172A] focus:outline-none focus:ring-2 focus:ring-[#3B82F6]/30 focus:border-[#3B82F6]/50 transition-all placeholder-slate-300",
                            placeholder: "e.g. web, stealth, office",
                            value: "{domain_input}",
                            oninput: move |e| domain_input.set(e.value())
                        }
                    }
                }
                div { class: "flex gap-3 pt-4",
                    button { 
                        class: "flex-1 px-4 py-2.5 bg-slate-100 hover:bg-slate-200 text-slate-600 rounded-lg transition-colors font-medium",
                        onclick: move |_| show_modal.set(false),
                        "Cancel"
                    }
                    button { 
                        class: "flex-1 px-4 py-2.5 bg-[#3B82F6] hover:bg-blue-500 text-white rounded-lg transition-colors font-medium shadow-sm",
                        onclick: move |_| handle_save.call(()),
                        "Save Changes"
                    }
                }
            }
        }
    }
}