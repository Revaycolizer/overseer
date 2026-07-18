use dioxus::prelude::*;

#[component]
pub fn ExecutionLog() -> Element {
    rsx! {
        aside { class: "w-80 border-r border-slate-200 bg-slate-50/80 flex flex-col min-h-0 overflow-y-auto custom-scrollbar p-5 space-y-6 text-xs text-slate-500 font-mono",
            div { class: "flex items-center justify-between border-b border-slate-200 pb-3",
                div { class: "flex items-center gap-2",
                    dioxus_material_icons::MaterialIcon { name: "terminal", size: 14, color: dioxus_material_icons::MaterialIconColor::Custom("#2563eb".into()) }
                    span { class: "font-black tracking-wider text-slate-700 uppercase", "Execution Log" }
                }
                span { class: "text-[9px] bg-slate-200/60 border border-slate-300/40 px-1.5 py-0.5 rounded text-slate-500", "v2.4.0" }
            }

            // Entry 1
            div { class: "space-y-2 border-l border-slate-200 pl-4 relative",
                div { class: "absolute -left-[4.5px] top-1.5 w-2 h-2 rounded-full bg-blue-600 border border-slate-50" }
                p { class: "text-[10px] text-slate-400 flex items-center gap-1",
                    span { "[14:22:01]" }
                    span { class: "text-blue-600 font-bold", "INITIATE" }
                }
                p { class: "text-slate-700 leading-relaxed font-medium", "Session: \"Market Analysis Automation\"" }
            }

            // Entry 2
            div { class: "space-y-2 border-l border-slate-200 pl-4 relative",
                div { class: "absolute -left-[4.5px] top-1.5 w-2 h-2 rounded-full bg-emerald-500 border border-slate-50" }
                p { class: "text-[10px] text-slate-400 flex items-center gap-1",
                    span { "[14:22:05]" }
                    span { class: "text-emerald-600 font-bold", "ACTIVE" }
                }
                p { class: "text-slate-700 font-medium", "AGENT: Opening Google Chrome..." }
                div { class: "bg-white border border-slate-200 rounded-xl p-2.5 flex items-center justify-between mt-1 text-slate-500 shadow-sm",
                    span { class: "italic", "Searching..." }
                    dioxus_material_icons::MaterialIcon { name: "search", size: 12 }
                }
            }

            // Entry 3
            div { class: "space-y-2 border-l border-slate-200 pl-4 relative",
                div { class: "absolute -left-[4.5px] top-1.5 w-2 h-2 rounded-full bg-indigo-500 border border-slate-50" }
                p { class: "text-[10px] text-slate-400 flex items-center gap-1",
                    span { "[14:23:12]" }
                    span { class: "text-indigo-600 font-bold", "PROCESS" }
                }
                p { class: "text-slate-700 font-medium", "AGENT: Extracting tabular data..." }
                div { class: "text-[10px] leading-relaxed text-indigo-600 font-semibold",
                    p { "> mapping: table_data_01" }
                    p { "> mapping: column_header_id" }
                }
                div { class: "bg-blue-50/60 border border-blue-100 rounded-xl p-3 space-y-2 mt-1 shadow-sm",
                    div { class: "flex items-center gap-1.5 text-blue-600 font-medium",
                        dioxus_material_icons::MaterialIcon { name: "info_outline", size: 12 }
                        span { "Mismatch detected. Auto-patching with script v2.1..." }
                    }
                    div { class: "flex gap-1 text-[9px] font-bold uppercase",
                        span { class: "bg-blue-100 text-blue-600 px-1.5 py-0.5 rounded", "JS_INJECT" }
                        span { class: "bg-emerald-100 text-emerald-600 px-1.5 py-0.5 rounded", "REPAIRED" }
                    }
                }
            }

            // Entry 4
            div { class: "space-y-2 border-l border-slate-200 pl-4 relative",
                div { class: "absolute -left-[4.5px] top-1.5 w-2 h-2 rounded-full bg-amber-500 border border-slate-50 animate-pulse" }
                p { class: "text-[10px] text-slate-400 flex items-center gap-1",
                    span { "[14:24:00]" }
                    span { class: "text-amber-600 font-bold animate-pulse", "AWAIT_INPUT" }
                }
                p { class: "text-slate-700 font-medium", "AGENT: Requesting manual review of Cell B42..." }
            }
        }
    }
}
