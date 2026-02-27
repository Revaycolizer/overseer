use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

pub fn AppManager() -> Element {
    rsx! {
        main { class: "flex-1 overflow-y-auto custom-scrollbar p-8",
            div { class: "mb-10 flex flex-col md:flex-row md:items-end justify-between gap-6",
                div {
                    h2 { class: "text-3xl font-extrabold tracking-tight text-slate-900 mb-2",
                        "App Integrations"
                    }
                    p { class: "text-base text-slate-500 max-w-2xl",
                        " Connect your local LLMs to your favorite desktop applications and system tools. "
                    }
                }
                div { class: "flex items-center gap-3",
                    div { class: "relative group",
                        div { class: "absolute left-3 top-1/2 -translate-y-1/2 flex items-center",
                            MaterialIcon { 
                                name: "search", 
                                color: MaterialIconColor::Custom("text-slate-400".to_string()),
                                size: 18,
                            }
                        }
                        input {
                            class: "bg-white border-slate-200 rounded-lg text-sm w-64 focus:ring-1 focus:ring-brand-blue/20 focus:border-brand-blue transition-all py-2 pl-9 shadow-sm",
                            placeholder: "Search integrations...",
                            r#type: "text",
                        }
                    }
                    button { class: "bg-primary text-white px-4 py-2 rounded-lg text-sm font-semibold flex items-center gap-2 hover:bg-slate-800 shadow-sm transition-all active:scale-[0.98]",
                        MaterialIcon { name: "sync", color: MaterialIconColor::Light, size: 18 }
                        " Scan "
                    }
                }
            }
            div { class: "flex flex-wrap items-center justify-between gap-4 mb-8",
                div { class: "flex flex-wrap gap-2",
                    button { class: "bg-slate-900 text-white px-4 py-1.5 rounded-full text-xs font-semibold shadow-sm",
                        "All Integrations"
                    }
                    button { class: "bg-white border border-slate-200 text-slate-500 px-4 py-1.5 rounded-full text-xs font-medium hover:border-slate-300 transition-colors",
                        "Active (8)"
                    }
                    button { class: "bg-white border border-slate-200 text-slate-500 px-4 py-1.5 rounded-full text-xs font-medium hover:border-slate-300 transition-colors",
                        "Idle (3)"
                    }
                    button { class: "bg-white border border-slate-200 text-slate-500 px-4 py-1.5 rounded-full text-xs font-medium hover:border-slate-300 transition-colors",
                        "Warnings (1)"
                    }
                }
                div { class: "text-[10px] text-slate-400 font-bold uppercase tracking-widest",
                    " 12 Results Found "
                }
            }
            div { class: "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-6",
                div { class: "bg-card-bg border border-slate-200 rounded-xl shadow-sm hover:shadow-md transition-all flex flex-col group",
                    div { class: "p-6 flex-1",
                        div { class: "flex items-start justify-between mb-5",
                            div { class: "flex items-center gap-4",
                                div { class: "w-12 h-12 bg-slate-50 rounded-xl flex items-center justify-center border border-slate-100 group-hover:bg-white transition-colors",
                                    MaterialIcon { name: "code", color: MaterialIconColor::Custom("text-slate-500".to_string()), size: 24 }
                                }
                                div {
                                    h3 { class: "font-bold text-slate-900 text-lg leading-tight",
                                        "Code Editor"
                                    }
                                    div { class: "flex items-center gap-1.5 mt-1",
                                        span { class: "w-2 h-2 rounded-full bg-status-active" }
                                        span { class: "text-[10px] font-bold text-status-active uppercase tracking-wide",
                                            "Active"
                                        }
                                    }
                                }
                            }
                            button { class: "text-slate-300 hover:text-slate-600 transition-colors flex items-center",
                                MaterialIcon { name: "more_horiz", size: 24 }
                            }
                        }
                        div { class: "space-y-5",
                            div {
                                p { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2.5",
                                    "Permissions"
                                }
                                div { class: "flex flex-wrap gap-1.5",
                                    div { class: "bg-slate-50 border border-slate-100 text-slate-600 px-2 py-1 rounded text-[10px] font-medium flex items-center gap-1",
                                        MaterialIcon { name: "visibility", size: 12 }
                                        " Read "
                                    }
                                    div { class: "bg-slate-50 border border-slate-100 text-slate-600 px-2 py-1 rounded text-[10px] font-medium flex items-center gap-1",
                                        MaterialIcon { name: "edit", size: 12 }
                                        " Write "
                                    }
                                }
                            }
                            div { class: "bg-slate-50/50 rounded-lg border border-slate-100 p-3",
                                p { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2",
                                    "Recent Activity"
                                }
                                ul { class: "space-y-2",
                                    li { class: "flex items-start gap-2 text-[11px] text-slate-600",
                                        span { class: "text-slate-400 font-mono",
                                            "14:22"
                                        }
                                        span { "Refactored data validation layer" }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "border-t border-slate-100 p-4 bg-slate-50/30 flex justify-between items-center rounded-b-xl",
                        button { class: "text-xs font-bold text-slate-400 hover:text-red-500 uppercase transition-colors",
                            "Disconnect"
                        }
                        button { class: "text-xs font-bold text-brand-blue hover:text-blue-700 uppercase transition-colors",
                            "Configure"
                        }
                    }
                }
                div { class: "bg-card-bg border border-slate-200 rounded-xl shadow-sm hover:shadow-md transition-all flex flex-col group",
                    div { class: "p-6 flex-1",
                        div { class: "flex items-start justify-between mb-5",
                            div { class: "flex items-center gap-4",
                                div { class: "w-12 h-12 bg-slate-50 rounded-xl flex items-center justify-center border border-slate-100 group-hover:bg-white transition-colors",
                                    MaterialIcon { name: "mail", color: MaterialIconColor::Custom("text-slate-500".to_string()), size: 24 }
                                }
                                div {
                                    h3 { class: "font-bold text-slate-900 text-lg leading-tight",
                                        "Communications"
                                    }
                                    div { class: "flex items-center gap-1.5 mt-1",
                                        span { class: "w-2 h-2 rounded-full bg-status-idle" }
                                        span { class: "text-[10px] font-bold text-status-idle uppercase tracking-wide",
                                            "Idle"
                                        }
                                    }
                                }
                            }
                            button { class: "text-slate-300 hover:text-slate-600 transition-colors flex items-center",
                                MaterialIcon { name: "more_horiz", size: 24 }
                            }
                        }
                        div { class: "space-y-5",
                            div {
                                p { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2.5",
                                    "Permissions"
                                }
                                div { class: "flex flex-wrap gap-1.5",
                                    div { class: "bg-slate-50 border border-slate-100 text-slate-600 px-2 py-1 rounded text-[10px] font-medium flex items-center gap-1",
                                        MaterialIcon { name: "visibility", size: 12 }
                                        " Read "
                                    }
                                }
                            }
                            div { class: "bg-slate-50/50 rounded-lg border border-slate-100 p-3",
                                p { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-2",
                                    "Recent Activity"
                                }
                                p { class: "text-[11px] text-slate-500 italic",
                                    "No recent activity found."
                                }
                            }
                        }
                    }
                    div { class: "border-t border-slate-100 p-4 bg-slate-50/30 flex justify-between items-center rounded-b-xl",
                        button { class: "text-xs font-bold text-slate-400 hover:text-red-500 uppercase transition-colors",
                            "Disconnect"
                        }
                        button { class: "text-xs font-bold text-brand-blue hover:text-blue-700 uppercase transition-colors",
                            "Configure"
                        }
                    }
                }
                div { class: "group border-2 border-dashed border-slate-200 rounded-xl flex flex-col items-center justify-center p-8 gap-4 hover:border-brand-blue/30 hover:bg-blue-50/30 transition-all cursor-pointer min-h-[300px]",
                    div { class: "w-14 h-14 rounded-full bg-white border border-slate-200 flex items-center justify-center group-hover:scale-110 transition-transform shadow-sm",
                        MaterialIcon { name: "add", color: MaterialIconColor::Custom("text-slate-400".to_string()), size: 24 }
                    }
                    div { class: "text-center",
                        h3 { class: "font-bold text-base text-slate-900", "Add Custom Tool" }
                        p { class: "text-sm text-slate-500 max-w-[200px] mt-2 leading-relaxed",
                            "Connect external CLI scripts, legacy tools, or custom APIs."
                        }
                    }
                }
            }
            div { class: "mt-16",
                div { class: "flex items-center justify-between mb-6",
                    div {
                        h3 { class: "text-xl font-bold tracking-tight text-slate-900",
                            "Integration Activity"
                        }
                        p { class: "text-sm text-slate-500 mt-1",
                            "Real-time log of communication between models and tools."
                        }
                    }
                    button { class: "text-brand-blue font-bold text-xs hover:text-blue-700 flex items-center gap-1.5 bg-blue-50 px-3 py-2 rounded-md transition-colors",
                        MaterialIcon { name: "download", size: 14 }
                        " Export CSV "
                    }
                }
                div { class: "bg-white border border-slate-200 rounded-xl overflow-hidden shadow-sm mb-8",
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-left text-xs",
                            thead {
                                tr { class: "bg-slate-50/80 text-slate-500 font-bold border-b border-slate-200",
                                    th { class: "px-6 py-4 uppercase tracking-wider",
                                        "Operation"
                                    }
                                    th { class: "px-6 py-4 uppercase tracking-wider",
                                        "Integration"
                                    }
                                    th { class: "px-6 py-4 uppercase tracking-wider",
                                        "Status"
                                    }
                                    th { class: "px-6 py-4 uppercase tracking-wider",
                                        "Resource"
                                    }
                                    th { class: "px-6 py-4 uppercase tracking-wider",
                                        "Timestamp"
                                    }
                                }
                            }
                            tbody { class: "divide-y divide-slate-100",
                                tr { class: "hover:bg-slate-50/50 transition-colors",
                                    td { class: "px-6 py-4 font-semibold text-slate-900",
                                        "Batch Document Parsing"
                                    }
                                    td { class: "px-6 py-4 text-slate-600 flex items-center gap-2",
                                        MaterialIcon { name: "code", color: MaterialIconColor::Custom("text-slate-400".to_string()), size: 18 }
                                        " Editor "
                                    }
                                    td { class: "px-6 py-4",
                                        span { class: "px-2.5 py-1 bg-emerald-50 text-emerald-700 rounded-md border border-emerald-100 text-[10px] font-bold uppercase",
                                            "Completed"
                                        }
                                    }
                                    td { class: "px-6 py-4 text-slate-500 italic",
                                        "Local Model (v3.2)"
                                    }
                                    td { class: "px-6 py-4 text-slate-400",
                                        "2 mins ago"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            footer { class: "py-12 border-t border-slate-200 mt-8",
                div { class: "flex flex-col md:flex-row justify-between items-center gap-8",
                    div { class: "flex items-center gap-2 text-slate-400",
                        MaterialIcon { name: "verified_user", size: 18 }
                        span { class: "text-[11px] font-bold uppercase tracking-widest",
                            "Secure Local Environment • v2.4.0"
                        }
                    }
                    div { class: "flex gap-10",
                        a {
                            class: "text-[11px] font-bold text-slate-400 hover:text-slate-900 uppercase tracking-widest transition-colors",
                            href: "#",
                            "Documentation"
                        }
                        a {
                            class: "text-[11px] font-bold text-slate-400 hover:text-slate-900 uppercase tracking-widest transition-colors",
                            href: "#",
                            "Privacy Policy"
                        }
                    }
                    div { class: "text-[11px] text-slate-400 font-medium bg-slate-100 px-3 py-1 rounded-full",
                        " Status: Optimal "
                    }
                }
            }
        }
    }
}