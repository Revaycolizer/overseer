use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

fn get_event_details(event: &crate::backend::ExecutionEvent) -> (String, String, String, String) {
    match event.event_type.as_str() {
        "tool_call" => {
            let tname = event.tool_name.as_deref().unwrap_or("unknown");
            (format!("Tool Call: {}", tname), "RUNNING".to_string(), "bg-blue-100 text-blue-700".to_string(), event.details.clone().unwrap_or_default())
        }
        "tool_result" => {
            let tname = event.tool_name.as_deref().unwrap_or("unknown");
            let is_ok = event.status == "completed";
            let badge = if is_ok { "SUCCESS" } else { "FAILED" };
            let color = if is_ok { "bg-emerald-100 text-emerald-700" } else { "bg-red-100 text-red-700" };
            let raw_details = event.details.clone().unwrap_or_default();
            let details = if raw_details.len() > 150 { format!("{}...", &raw_details[..150]) } else { raw_details };
            (format!("Tool Result: {}", tname), badge.to_string(), color.to_string(), details)
        }
        "micro_step" => {
            ("Micro Step".to_string(), "ACTIVE".to_string(), "bg-indigo-100 text-indigo-700".to_string(), event.step_text.clone().unwrap_or_default())
        }
        _ => {
            ("System Event".to_string(), event.status.to_uppercase(), "bg-slate-100 text-slate-700".to_string(), event.message.clone().unwrap_or_default())
        }
    }
}

#[component]
pub fn InteractionLog(events: Vec<crate::backend::ExecutionEvent>) -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 bg-slate-50/80 border-r border-slate-200 p-6 space-y-6 overflow-y-auto custom-scrollbar",
            div { class: "flex items-center justify-between border-b border-slate-200 pb-3 shrink-0",
                div { class: "flex items-center gap-2",
                    MaterialIcon { name: "terminal", size: 16, color: dioxus_material_icons::MaterialIconColor::Custom("#2563eb".into()) }
                    span { class: "text-xs font-black tracking-wider text-slate-800 uppercase font-mono", "Interaction Log" }
                    span { class: "text-[9px] bg-slate-200/60 border border-slate-300/40 px-1.5 py-0.5 rounded text-slate-500 font-mono", "v2.4.0" }
                }
            }

            div { class: "space-y-4 flex-1 min-h-0",
                if events.is_empty() {
                    div { class: "text-center py-8 text-slate-400 font-mono text-xs", "No interaction events recorded for this session." }
                } else {
                    for event in events.iter() {
                        {
                            let utc_time = chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(event.created_at, chrono::Utc);
                            let local_time = utc_time.with_timezone(&chrono::Local);
                            let formatted_time = local_time.format("%I:%M:%S %p").to_string();
                            let (title, badge, badge_class, details) = get_event_details(event);
                            rsx! {
                                div { class: "bg-white border border-slate-200 rounded-2xl p-4 space-y-2.5 shadow-sm hover:shadow-md transition-shadow",
                                    div { class: "flex items-center justify-between text-xs",
                                        div { class: "flex items-center gap-2",
                                            span { class: "font-black text-slate-800", "{title}" }
                                            span { class: "text-[8px] font-black px-1.5 py-0.5 rounded tracking-wide uppercase {badge_class}", "{badge}" }
                                        }
                                        span { class: "text-[10px] font-mono text-slate-400 font-bold", "{formatted_time}" }
                                    }
                                    if !details.is_empty() {
                                        p { class: "text-xs text-slate-600 leading-relaxed font-mono whitespace-pre-wrap break-all bg-slate-50 p-2 rounded-xl border border-slate-100", "{details}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
