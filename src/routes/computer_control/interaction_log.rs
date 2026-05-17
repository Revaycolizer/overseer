use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn InteractionLog() -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 bg-slate-50/80 border-r border-slate-200 p-6 space-y-6 overflow-y-auto custom-scrollbar",
            // Header
            div { class: "flex items-center justify-between border-b border-slate-200 pb-3",
                div { class: "flex items-center gap-2",
                    MaterialIcon { name: "terminal", size: 16, color: dioxus_material_icons::MaterialIconColor::Custom("#2563eb".into()) }
                    span { class: "text-xs font-black tracking-wider text-slate-800 uppercase font-mono", "Interaction Log" }
                    span { class: "text-[9px] bg-slate-200/60 border border-slate-300/40 px-1.5 py-0.5 rounded text-slate-500 font-mono", "v2.4.0" }
                }
                div { class: "flex items-center gap-4 text-xs font-bold text-slate-500 font-mono",
                    span { class: "hover:text-blue-600 cursor-pointer flex items-center gap-1", MaterialIcon { name: "filter_alt", size: 14 } "Filter" }
                    span { class: "hover:text-blue-600 cursor-pointer flex items-center gap-1", MaterialIcon { name: "download", size: 14 } "Export" }
                }
            }

            // Timeline Feed
            div { class: "space-y-4",
                // Bash Execution Card
                div { class: "bg-white border border-slate-200 rounded-2xl p-5 space-y-3.5 shadow-sm hover:shadow-md transition-shadow",
                    div { class: "flex items-center justify-between text-xs",
                        div { class: "flex items-center gap-3",
                            div { class: "bg-slate-100 p-2 rounded-xl text-slate-600 flex items-center justify-center border border-slate-200/50", MaterialIcon { name: "integration_instructions", size: 16 } }
                            div { class: "flex items-center gap-2",
                                span { class: "font-black text-slate-800", "Bash Execution" }
                                span { class: "bg-emerald-100 text-emerald-700 text-[8px] font-black px-1.5 py-0.5 rounded tracking-wide uppercase", "SUCCESS" }
                                span { class: "bg-slate-100 text-slate-600 text-[8px] font-black px-1.5 py-0.5 rounded tracking-wide uppercase border border-slate-200", "BUILD" }
                            }
                        }
                        span { class: "text-[10px] font-mono text-slate-400 font-bold", "12:44:12 PM" }
                    }
                    p { class: "text-xs text-slate-600 leading-relaxed font-medium", "Executed `npm run build` in `/projects/forge-ui`. Optimization pass completed in 4.2s." }
                    // Dark Bash Box
                    div { class: "bg-slate-950 border border-slate-900 rounded-xl p-3.5 font-mono text-[10px] text-blue-400 space-y-1 shadow-inner",
                        p { class: "text-slate-500", "$ npm run build" }
                        p { "> Asset size: 2.1MB (-14% reduction)" }
                        p { "> Gzip size: 452KB" }
                    }
                }

                // Document Synthesis Card
                div { class: "bg-white border border-slate-200 rounded-2xl p-5 space-y-3 shadow-sm hover:shadow-md transition-shadow",
                    div { class: "flex items-center justify-between text-xs",
                        div { class: "flex items-center gap-3",
                            div { class: "bg-slate-100 p-2 rounded-xl text-slate-600 flex items-center justify-center border border-slate-200/50", MaterialIcon { name: "description", size: 16 } }
                            div { class: "flex items-center gap-2",
                                span { class: "font-black text-slate-800", "Document Synthesis" }
                                span { class: "bg-slate-900 text-white text-[8px] font-black px-1.5 py-0.5 rounded tracking-wide uppercase", "PROCESSING" }
                            }
                        }
                        span { class: "text-[10px] font-mono text-slate-400 font-bold", "12:15:08 PM" }
                    }
                    p { class: "text-xs text-slate-600 leading-relaxed font-medium", "Generated architecture summary for the 'Slate' design system documentation. 14 pages processed with 98% accuracy. Referenced <a class='text-blue-600 underline' href='#'>https://lh3.googleusercontent.com/...</a> for brand assets." }
                }

                // Web Navigation Card
                div { class: "bg-white border border-slate-200 rounded-2xl p-5 space-y-3 shadow-sm hover:shadow-md transition-shadow",
                    div { class: "flex items-center justify-between text-xs",
                        div { class: "flex items-center gap-3",
                            div { class: "bg-slate-100 p-2 rounded-xl text-slate-600 flex items-center justify-center border border-slate-200/50", MaterialIcon { name: "language", size: 16 } }
                            div { class: "flex items-center gap-2",
                                span { class: "font-black text-slate-800", "Web Navigation" }
                                span { class: "bg-slate-100 text-slate-600 text-[8px] font-black px-1.5 py-0.5 rounded tracking-wide uppercase border border-slate-200", "COMPLETE" }
                            }
                        }
                        span { class: "text-[10px] font-mono text-slate-400 font-bold", "11:58:32 AM" }
                    }
                    p { class: "text-xs text-slate-600 leading-relaxed font-medium", "Browsed MDN Web Docs to verify CSS backdrop-filter compatibility across Chromium-based browsers. Summary attached to internal knowledge base." }
                }

                // Node Handshake Card
                div { class: "bg-white border border-slate-200 rounded-2xl p-5 space-y-3 shadow-sm hover:shadow-md transition-shadow",
                    div { class: "flex items-center justify-between text-xs",
                        div { class: "flex items-center gap-3",
                            div { class: "bg-slate-100 p-2 rounded-xl text-slate-600 flex items-center justify-center border border-slate-200/50", MaterialIcon { name: "router", size: 16 } }
                            div { class: "flex items-center gap-2",
                                span { class: "font-black text-slate-800", "Node Handshake" }
                                span { class: "bg-emerald-100 text-emerald-700 text-[8px] font-black px-1.5 py-0.5 rounded tracking-wide uppercase", "SUCCESS" }
                            }
                        }
                        span { class: "text-[10px] font-mono text-slate-400 font-bold", "11:30:15 AM" }
                    }
                    p { class: "text-xs text-slate-600 leading-relaxed font-medium", "Established secure tunnel with Forge Cluster US-East. Latency: 12ms." }
                }
            }
        }
    }
}
