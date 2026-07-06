use dioxus::prelude::*;

#[component]
pub fn HeaderBar(
    mut active_widgets: Signal<Vec<String>>,
    mut show_customizer: Signal<bool>,
) -> Element {
    rsx! {
        div { class: "h-14 border-b border-slate-200 flex items-center justify-between px-6 bg-white shrink-0 relative z-30",
            div { class: "flex items-center gap-3",
                span { class: "text-lg font-black tracking-wider bg-gradient-to-r from-blue-600 to-indigo-600 bg-clip-text text-transparent uppercase", "Personal AI" }
                div { class: "relative flex items-center bg-slate-50 border border-slate-200 rounded-xl px-3 py-1.5 w-80 group",
                    dioxus_material_icons::MaterialIcon { name: "search", size: 16, color: dioxus_material_icons::MaterialIconColor::Custom("#94a3b8".into()) }
                    input {
                        class: "bg-transparent border-0 outline-none text-xs text-slate-700 ml-2 placeholder:text-slate-400 w-full",
                        placeholder: "Search logs, tasks, or nodes..."
                    }
                }
            }
            
            div { class: "flex items-center gap-4 text-xs font-bold text-slate-500",
                div { class: "relative",
                    button {
                        class: "bg-slate-50 hover:bg-slate-100 border border-slate-200 px-3.5 py-1.5 rounded-xl flex items-center gap-1.5 transition-all text-[11px] uppercase tracking-wider text-slate-700 shadow-sm",
                        onclick: move |_| show_customizer.toggle(),
                        dioxus_material_icons::MaterialIcon { name: "dashboard_customize", size: 14 }
                        "Customize View"
                    }
                    if show_customizer() {
                        div { class: "absolute right-0 mt-2 w-56 bg-white border border-slate-200 rounded-2xl p-4 shadow-xl z-50 space-y-2.5 text-xs text-slate-700",
                            p { class: "font-black text-slate-800 uppercase tracking-wider text-[10px] pb-1 border-b border-slate-100", "Toggle Widgets" }
                            for name in ["Interaction Log", "Control Center", "System Load", "GPU Performance", "Background Processes", "Environment"] {
                                label { class: "flex items-center gap-2.5 py-1 hover:bg-slate-50 px-1 rounded cursor-pointer transition-colors",
                                    input {
                                        r#type: "checkbox",
                                        class: "accent-blue-600 w-4 h-4 rounded border-slate-300",
                                        checked: active_widgets.read().contains(&name.to_string()),
                                        onchange: move |e| {
                                            let mut list = active_widgets.read().clone();
                                            if e.value() == "true" {
                                                if !list.contains(&name.to_string()) { list.push(name.to_string()); }
                                            } else {
                                                list.retain(|x| x != name);
                                            }
                                            active_widgets.set(list);
                                        }
                                    }
                                    span { class: "font-medium", "{name}" }
                                }
                            }
                        }
                    }
                }

                div { class: "flex items-center gap-2 bg-slate-50 border border-slate-200 rounded-lg px-2 py-1 text-slate-700",
                    span { class: "w-1.5 h-1.5 rounded-full bg-blue-600 animate-pulse" }
                    "ADMIN NODE"
                    span { class: "bg-blue-600/10 text-blue-600 text-[10px] px-1 rounded", "PRO" }
                }
                dioxus_material_icons::MaterialIcon { name: "notifications", size: 18 }
            }
        }
    }
}
