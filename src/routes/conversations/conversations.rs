use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn Conversations() -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col min-w-0 relative",
            header { class: "h-16 shrink-0 border-b border-slate-200 bg-white/80 backdrop-blur-md flex items-center justify-between px-8 z-10",
                div { class: "flex items-center gap-6",
                    h1 { class: "text-xl font-bold tracking-tight text-[var(--text-main)]",
                        "Overseer"
                    }
                    div { class: "flex items-center gap-2 px-3 py-1 bg-slate-100 border border-slate-200 rounded-full",
                        div { class: "size-2 bg-emerald-500 rounded-full" }
                        span { class: "text-[10px] font-bold text-slate-500 uppercase tracking-widest",
                            "Llama 3 Local"
                        }
                    }
                }
                div { class: "flex items-center gap-4",
                    div { class: "flex items-center gap-2 text-slate-500 text-[11px] font-medium uppercase tracking-wider",
                        MaterialIcon { name: "storage", size: 16 }
                        span { "Knowledge: Local Repo" }
                    }
                    div { class: "h-4 w-px bg-slate-200 mx-2" }
                    button { class: "text-slate-400 hover:text-slate-600 transition-colors flex items-center",
                        MaterialIcon { name: "search", size: 24 }
                    }
                    button { class: "text-slate-400 hover:text-slate-600 transition-colors flex items-center",
                        MaterialIcon { name: "notifications", size: 24 }
                    }
                }
            }
            div { class: "flex flex-1 overflow-hidden",
                aside { class: "w-72 border-r border-slate-200 bg-white flex flex-col shrink-0",
                    div { class: "p-5",
                        button { class: "w-full flex items-center justify-center gap-2 bg-blue-600 hover:bg-blue-700 text-white font-semibold py-2.5 rounded-lg shadow-soft transition-all text-xs",
                            MaterialIcon { name: "add", color: MaterialIconColor::Light, size: 20 }
                            span { "New Conversation" }
                        }
                    }
                    div { class: "flex-1 overflow-y-auto px-4 space-y-6",
                        div {
                            h3 { class: "px-2 text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3",
                                "Today"
                            }
                            div { class: "space-y-1",
                                a {
                                    class: "flex items-center gap-3 px-3 py-2 bg-blue-50 text-blue-700 rounded-lg border border-blue-100",
                                    href: "#",
                                    MaterialIcon { name: "terminal", size: 20 }
                                    span { class: "text-xs font-semibold truncate",
                                        "Local API Debugging"
                                    }
                                }
                                a {
                                    class: "flex items-center gap-3 px-3 py-2 hover:bg-slate-50 rounded-lg transition-colors text-slate-600 group",
                                    href: "#",
                                    MaterialIcon { name: "chat_bubble", size: 20 }
                                    span { class: "text-xs font-medium truncate group-hover:text-slate-900",
                                        "System Optimization Discussion"
                                    }
                                }
                            }
                        }
                        div {
                            h3 { class: "px-2 text-[10px] font-bold text-slate-400 uppercase tracking-widest mb-3",
                                "Yesterday"
                            }
                            div { class: "space-y-1",
                                a {
                                    class: "flex items-center gap-3 px-3 py-2 hover:bg-slate-50 rounded-lg transition-colors text-slate-600 group",
                                    href: "#",
                                    MaterialIcon { name: "description", size: 20 }
                                    span { class: "text-xs font-medium truncate group-hover:text-slate-900",
                                        "Database Schema Proposal"
                                    }
                                }
                            }
                        }
                    }
                    div { class: "p-4 bg-slate-50 border-t border-slate-200",
                        div { class: "bg-white rounded-lg p-3 border border-slate-200 shadow-sm",
                            div { class: "flex items-center justify-between mb-2",
                                span { class: "text-[10px] font-bold text-slate-500 uppercase",
                                    "GPU Load"
                                }
                                span { class: "text-[10px] font-mono text-blue-600 font-bold",
                                    "42%"
                                }
                            }
                            div { class: "h-1.5 w-full bg-slate-100 rounded-full overflow-hidden",
                                div { class: "h-full bg-blue-500 w-[42%]" }
                            }
                        }
                    }
                }
                main { class: "flex-1 flex flex-col relative bg-[var(--workspace-bg)]",
                    div { class: "flex-1 overflow-y-auto",
                        div { class: "max-w-3xl mx-auto py-10 px-6 space-y-8",
                            div { class: "flex items-start gap-4",
                                div { class: "size-8 shrink-0 rounded-lg bg-blue-600 flex items-center justify-center text-white shadow-soft",
                                    MaterialIcon { name: "smart_toy", color: MaterialIconColor::Light, size: 20 }
                                }
                                div { class: "space-y-2 flex-1",
                                    div { class: "flex items-center gap-2",
                                        span { class: "text-[11px] font-bold uppercase tracking-widest text-slate-500",
                                            "Overseer"
                                        }
                                        span { class: "text-[9px] bg-white text-blue-600 border border-blue-100 px-1.5 py-0.5 rounded font-bold uppercase tracking-tighter",
                                            "SECURE"
                                        }
                                    }
                                    div { class: "bg-[var(--assistant-bubble)] p-4 rounded-2xl rounded-tl-none border border-blue-50/50 text-[var(--text-main)] text-sm leading-relaxed shadow-bubble",
                                        p {
                                            "I have analyzed the local development context. The current API middleware is showing a bottleneck in the authentication layer. Would you like me to suggest a refactor or continue monitoring the traffic logs?"
                                        }
                                    }
                                }
                            }
                            div { class: "flex items-start gap-4 flex-row-reverse",
                                div { class: "size-8 shrink-0 rounded-lg border border-slate-200 bg-white flex items-center justify-center overflow-hidden shadow-soft",
                                    img {
                                        alt: "User Avatar",
                                        class: "w-full h-full object-cover",
                                        src: "https://lh3.googleusercontent.com/aida-public/AB6AXuBqMBjxuGBSeYlBYQz3ehCFqzRwa9Wyq9eVwMYy09GJ935PrL_9J9Nm026Ma8JaIJeKfVWN7hdeIFL0gi4M4lOputd5eqoppoLRMUGbZnFachbwDjykRgdxvZCEKJojPy5zMgRnDuAjxuLndEN_6WYnHU9MPtepRB2SOYmZ_WbKPgUjNyE5Z8A9NuRt-cff_feV6mAy2VkbTR38GeWrNCj20LzICJCLsrMqsI68pBlMyS4UV1ORb1_pYK5tyDwjtOlKMwPP2uuBrtw",
                                    }
                                }
                                div { class: "space-y-2 flex-1 text-right",
                                    div { class: "flex items-center gap-2 justify-end",
                                        span { class: "text-[11px] font-bold uppercase tracking-widest text-slate-500",
                                            "Executive User"
                                        }
                                    }
                                    div { class: "bg-[var(--user-bubble)] p-4 rounded-2xl rounded-tr-none text-[var(--text-main)] text-sm leading-relaxed shadow-bubble border border-slate-200 inline-block text-left max-w-[85%]",
                                        p {
                                            "Suggest the refactor. Prioritize reducing the latency for the handshake protocol."
                                        }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "p-6 bg-gradient-to-t from-[var(--workspace-bg)] via-[var(--workspace-bg)] to-transparent",
                        div { class: "max-w-3xl mx-auto",
                            div { class: "relative bg-white border border-slate-200 rounded-xl p-2 shadow-xl focus-within:border-blue-400 focus-within:ring-4 focus-within:ring-blue-500/5 transition-all",
                                div { class: "flex items-end gap-2",
                                    button { class: "p-2.5 text-slate-400 hover:text-slate-600 transition-colors bg-slate-50 rounded-lg border border-slate-100 group flex items-center",
                                        MaterialIcon { name: "attach_file", size: 24 }
                                    }
                                    textarea {
                                        class: "flex-1 bg-transparent border-none focus:ring-0 text-[var(--text-main)] placeholder:text-slate-400 text-sm py-3 px-2 resize-none max-h-48",
                                        placeholder: "Command assistant...",
                                        rows: "1",
                                    }
                                    button { class: "size-11 flex items-center justify-center bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-all shadow-lg shadow-blue-500/20",
                                        MaterialIcon { name: "arrow_upward", color: MaterialIconColor::Light, size: 24 }
                                    }
                                }
                            }
                            div { class: "mt-4 flex items-center justify-center gap-4",
                                span { class: "text-[10px] text-slate-400 font-bold uppercase tracking-widest",
                                    "Model: Llama-3-8B-Q4_K_M"
                                }
                                div { class: "h-1 w-1 rounded-full bg-slate-300" }
                                span { class: "text-[10px] text-slate-400 font-bold uppercase tracking-widest",
                                    "Latency: 24ms"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}