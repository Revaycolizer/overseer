use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn WorkflowAutomation() -> Element {
    rsx! {
        
                main { class: "flex-1 overflow-y-auto custom-scrollbar p-8",
                    div { class: "mb-10 flex flex-col md:flex-row md:items-end justify-between gap-6",
                        div {
                            h2 { class: "text-3xl font-extrabold tracking-tight text-slate-900 mb-2",
                                "Workflow Automation"
                            }
                            p { class: "text-base text-slate-500 max-w-2xl",
                                " Record, edit, and schedule multi-step macros across your connected applications. "
                            }
                        }
                        div { class: "flex items-center gap-3",
                            button { class: "bg-red-500 text-white px-4 py-2 rounded-lg text-sm font-semibold flex items-center gap-2 hover:bg-red-600 shadow-sm transition-all active:scale-[0.98]",
                                MaterialIcon { name: "radio_button_checked", size: 28, color: MaterialIconColor::Custom("".to_string()) }
                                " Record New "
                            }
                        }
                    }
                    div { class: "grid grid-cols-1 lg:grid-cols-3 gap-8",
                        div { class: "lg:col-span-1 space-y-4",
                            div { class: "flex items-center justify-between mb-2",
                                h3 { class: "text-sm font-bold text-slate-400 uppercase tracking-widest",
                                    "Recent Macros"
                                }
                            }
                            div { class: "bg-white border-2 border-brand-blue rounded-xl p-4 shadow-sm cursor-pointer",
                                div { class: "flex justify-between items-start mb-3",
                                    div { class: "w-10 h-10 bg-blue-50 rounded-lg flex items-center justify-center",
                                        MaterialIcon { name: "summarize", size: 24, color: MaterialIconColor::Custom("text-brand-blue".to_string()) }
                                    }
                                    span { class: "text-[10px] font-bold text-brand-blue bg-blue-50 px-2 py-0.5 rounded uppercase",
                                        "Active"
                                    }
                                }
                                h4 { class: "font-bold text-slate-900", "Daily Report Sync" }
                                p { class: "text-xs text-slate-500 mt-1",
                                    "Extracts Jira tasks and posts to Slack daily."
                                }
                                div { class: "mt-4 pt-4 border-t border-slate-100 flex items-center justify-between",
                                    span { class: "text-[11px] text-slate-400", "8 steps • 2m 14s" }
                                    MaterialIcon { name: "play_circle", size: 28, color: MaterialIconColor::Custom("text-slate-400".to_string()) }
                                }
                            }
                            div { class: "bg-white border border-slate-200 rounded-xl p-4 shadow-sm hover:border-slate-300 cursor-pointer transition-colors",
                                div { class: "flex justify-between items-start mb-3",
                                    div { class: "w-10 h-10 bg-slate-50 rounded-lg flex items-center justify-center",
                                        MaterialIcon { name: "mail", size: 24, color: MaterialIconColor::Custom("text-slate-400".to_string()) }
                                    }
                                }
                                h4 { class: "font-bold text-slate-900", "Email Auto-Sorter" }
                                p { class: "text-xs text-slate-500 mt-1",
                                    "Categorizes incoming mail based on LLM analysis."
                                }
                                div { class: "mt-4 pt-4 border-t border-slate-100 flex items-center justify-between",
                                    span { class: "text-[11px] text-slate-400", "5 steps • 45s" }
                                    MaterialIcon { name: "play_circle", size: 28, color: MaterialIconColor::Custom("text-slate-400".to_string()) }
                                }
                            }
                        }
                        div { class: "lg:col-span-2",
                            div { class: "bg-white border border-slate-200 rounded-xl shadow-sm overflow-hidden h-full flex flex-col",
                                div { class: "p-4 border-b border-slate-100 flex items-center justify-between bg-slate-50/50",
                                    div { class: "flex items-center gap-3",
                                        h3 { class: "font-bold text-slate-900",
                                            "Editor: Daily Report Sync"
                                        }
                                        span { class: "bg-slate-200 w-[1px] h-4" }
                                        span { class: "text-xs text-slate-500",
                                            "Last edited 2 hours ago"
                                        }
                                    }
                                    div { class: "flex items-center gap-2",
                                        button { class: "p-2 text-slate-400 hover:text-slate-600 transition-colors",
                                            MaterialIcon { name: "settings", size: 32, color: MaterialIconColor::Custom("".to_string()) }
                                        }
                                        button { class: "bg-slate-900 text-white px-4 py-1.5 rounded-lg text-xs font-bold",
                                            "Save Changes"
                                        }
                                    }
                                }
                                div { class: "flex-1 p-8 overflow-y-auto custom-scrollbar",
                                    div { class: "relative ml-4 border-l-2 border-slate-100 pl-8 space-y-8",
                                        div { class: "relative macro-step-dot",
                                            div { class: "flex items-center gap-4 mb-2",
                                                span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                                                    "Step 1"
                                                }
                                                span { class: "bg-emerald-50 text-emerald-600 text-[10px] font-bold px-2 py-0.5 rounded uppercase",
                                                    "Trigger"
                                                }
                                            }
                                            div { class: "bg-slate-50 rounded-xl p-4 border border-slate-100",
                                                div { class: "flex items-center gap-3",
                                                    MaterialIcon { name: "schedule", size: 24, color: MaterialIconColor::Custom("text-slate-500".to_string()) }
                                                    div {
                                                        p { class: "text-sm font-semibold text-slate-900",
                                                            "Scheduled Trigger"
                                                        }
                                                        p { class: "text-xs text-slate-500",
                                                            "Runs every weekday at 09:00 AM"
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        div { class: "relative macro-step-dot",
                                            div { class: "flex items-center gap-4 mb-2",
                                                span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                                                    "Step 2"
                                                }
                                                span { class: "bg-blue-50 text-blue-600 text-[10px] font-bold px-2 py-0.5 rounded uppercase",
                                                    "Action"
                                                }
                                            }
                                            div { class: "bg-white rounded-xl p-4 border border-slate-200",
                                                div { class: "flex items-center justify-between",
                                                    div { class: "flex items-center gap-3",
                                                        MaterialIcon { name: "database", size: 24, color: MaterialIconColor::Custom("text-slate-500".to_string()) }
                                                        div {
                                                            p { class: "text-sm font-semibold text-slate-900",
                                                                "Fetch Jira Tickets"
                                                            }
                                                            p { class: "text-xs text-slate-500",
                                                                "Filter: status=\"In Progress\" AND assignee=currentUser()"
                                                            }
                                                        }
                                                    }
                                                    MaterialIcon { name: "drag_indicator", size: 24, color: MaterialIconColor::Custom("text-slate-300".to_string()) }
                                                }
                                            }
                                        }
                                        div { class: "relative macro-step-dot",
                                            div { class: "flex items-center gap-4 mb-2",
                                                span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                                                    "Step 3"
                                                }
                                                span { class: "bg-purple-50 text-purple-600 text-[10px] font-bold px-2 py-0.5 rounded uppercase",
                                                    "AI Logic"
                                                }
                                            }
                                            div { class: "bg-white rounded-xl p-4 border border-slate-200",
                                                div { class: "flex items-center justify-between",
                                                    div { class: "flex items-center gap-3",
                                                        MaterialIcon { name: "psychology", size: 24, color: MaterialIconColor::Custom("text-slate-500".to_string()) }
                                                        div {
                                                            p { class: "text-sm font-semibold text-slate-900",
                                                                "Summarize Content"
                                                            }
                                                            p { class: "text-xs text-slate-500",
                                                                "Prompt: \"Summarize these tickets into a bulleted list for Slack...\""
                                                            }
                                                        }
                                                    }
                                                    MaterialIcon { name: "drag_indicator", size: 24, color: MaterialIconColor::Custom("text-slate-300".to_string()) }
                                                }
                                            }
                                        }
                                        div { class: "relative",
                                            button { class: "flex items-center gap-2 text-brand-blue font-bold text-xs hover:bg-blue-50 p-2 -ml-2 rounded-lg transition-colors",
                                                MaterialIcon { name: "add_circle", size: 18, color: MaterialIconColor::Custom("".to_string()) }
                                                " ADD NEW STEP "
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    footer { class: "py-12 border-t border-slate-200 mt-12",
                        div { class: "flex flex-col md:flex-row justify-between items-center gap-8",
                            div { class: "flex items-center gap-2 text-slate-400",
                                MaterialIcon { name: "verified_user", size: 28, color: MaterialIconColor::Custom("".to_string()) }
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