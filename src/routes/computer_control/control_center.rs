use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

fn find_latest_screenshot(events: &[crate::backend::ExecutionEvent]) -> Option<String> {
    events.iter().rev().find_map(|e| {
        if e.event_type != "tool_result" || e.tool_name.as_deref() != Some("os_tool") { return None; }
        let val: serde_json::Value = serde_json::from_str(e.details.as_deref()?).ok()?;
        if val.get("action")?.as_str()? != "screenshot" { return None; }
        Some(val.get("path")?.as_str()?.to_string())
    })
}

fn find_latest_url(events: &[crate::backend::ExecutionEvent]) -> String {
    events.iter().rev().find_map(|e| {
        if e.event_type != "tool_call" || e.tool_name.as_deref() != Some("os_tool") { return None; }
        let val: serde_json::Value = serde_json::from_str(e.details.as_deref()?).ok()?;
        if val.get("action")?.as_str()? != "open_url" { return None; }
        Some(val.get("url")?.as_str()?.to_string())
    }).unwrap_or_else(|| "https://overseer.ai/session".to_string())
}

#[component]
pub fn ControlCenter(run_status: String, events: Vec<crate::backend::ExecutionEvent>) -> Element {
    let screenshot_path = find_latest_screenshot(&events);
    let active_url = find_latest_url(&events);
    
    let screenshot_src = use_resource(move || {
        let path = screenshot_path.clone();
        async move {
            let Some(p) = path else { return None; };
            match std::fs::read(&p) {
                Ok(bytes) if !bytes.is_empty() => {
                    use base64::{engine::general_purpose, Engine as _};
                    Some(format!("data:image/png;base64,{}", general_purpose::STANDARD.encode(bytes)))
                }
                _ => None,
            }
        }
    });

    let live_label = if run_status == "running" { "LIVE" } else { "IDLE" };
    let live_color = if run_status == "running" { "bg-blue-600" } else { "bg-slate-500" };

    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 bg-white border-r border-slate-200 p-6 space-y-6 overflow-y-auto custom-scrollbar",
            div { class: "flex items-start justify-between flex-wrap gap-4",
                div { class: "space-y-1.5",
                    div { class: "flex items-center gap-2 text-[10px] font-black uppercase tracking-wider text-slate-400 font-mono",
                        span { "SESSION ID: AGENT-402-DELTA" }
                        span { class: "w-1.5 h-1.5 rounded-full bg-emerald-500" }
                    }
                    h2 { class: "text-2xl font-black text-slate-800 tracking-tight", "Cross-App Control Center" }
                }
            }

            div { class: "flex-1 bg-slate-50 border border-slate-200 rounded-3xl overflow-hidden flex flex-col relative shadow-sm min-h-[480px]",
                div { class: "h-11 border-b border-slate-200 bg-slate-100/70 px-4 flex items-center justify-between text-slate-400",
                    div { class: "flex items-center gap-1.5",
                        div { class: "w-2.5 h-2.5 rounded-full bg-red-400/80" }
                        div { class: "w-2.5 h-2.5 rounded-full bg-yellow-400/80" }
                        div { class: "w-2.5 h-2.5 rounded-full bg-green-400/80" }
                    }
                    div { class: "flex-1 max-w-md mx-auto flex items-center bg-white border border-slate-200 rounded-xl px-3 py-1 font-mono text-[10px] text-slate-600 justify-between shadow-sm",
                        div { class: "flex items-center gap-2 truncate",
                            MaterialIcon { name: "lock", size: 10, color: dioxus_material_icons::MaterialIconColor::Custom("#10b981".into()) }
                            span { class: "truncate", "{active_url}" }
                        }
                        span { class: "text-white text-[8px] font-bold px-1.5 py-0.5 rounded tracking-widest uppercase {live_color}", "{live_label}" }
                    }
                    MaterialIcon { name: "videocam", size: 16 }
                }

                div { class: "flex-1 relative flex flex-col items-center justify-center p-6 bg-slate-950 select-none overflow-hidden",
                    div { class: "absolute inset-0 bg-[linear-gradient(to_right,#1e293b_1px,transparent_1px),linear-gradient(to_bottom,#1e293b_1px,transparent_1px)] bg-[size:4rem_4rem] opacity-35" }
                    if let Some(Some(src)) = screenshot_src.read().as_ref() {
                        img { src: "{src}", class: "max-w-full max-h-full object-contain rounded-2xl shadow-2xl border border-slate-800 z-10" }
                    } else {
                        div { class: "w-full max-w-md bg-slate-900/90 border border-slate-800/80 rounded-2xl p-6 space-y-4 backdrop-blur shadow-2xl relative z-10 text-center",
                            div { class: "flex justify-center", MaterialIcon { name: "computer", size: 48, color: dioxus_material_icons::MaterialIconColor::Custom("#64748b".into()) } }
                            h3 { class: "text-sm font-bold text-slate-350", "No Active Desktop View" }
                            p { class: "text-[10px] text-slate-500 font-mono uppercase tracking-widest", "Agent status: {run_status}" }
                        }
                    }
                }
            }
        }
    }
}
