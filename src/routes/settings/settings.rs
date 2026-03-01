use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn Settings() -> Element {
    rsx! {
        div { class: "flex-1 flex flex-col min-h-0 bg-background-light overflow-y-auto custom-scrollbar",
            main { class: "flex-1 p-8 max-w-5xl mx-auto w-full",
                div { class: "mb-10",
                    h2 { class: "text-3xl font-extrabold tracking-tight text-slate-900 mb-2",
                        "Settings"
                    }
                    p { class: "text-base text-slate-500",
                        "Configure your local AI environment and application preferences."
                    }
                }
                div { class: "settings-section",
                    div { class: "flex items-center gap-3 mb-6",
                        MaterialIcon { name: "palette", color: MaterialIconColor::Custom("text-slate-400".to_string()), size: 24 }
                        h3 { class: "text-lg font-bold text-slate-900", "Appearance" }
                    }
                    div { class: "space-y-6",
                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-sm font-semibold text-slate-900",
                                    "Theme Preference"
                                }
                                p { class: "text-xs text-slate-500",
                                    "Choose between light, dark, or system default."
                                }
                            }
                            select { class: "bg-white border-slate-200 rounded-lg text-sm w-40 focus:ring-1 focus:ring-brand-blue/20 focus:border-brand-blue py-2 px-3 shadow-sm",
                                option { "Light Mode" }
                                option { "Dark Mode" }
                                option { "System Default" }
                            }
                        }
                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-sm font-semibold text-slate-900",
                                    "Sidebar Density"
                                }
                                p { class: "text-xs text-slate-500",
                                    "Adjust the spacing of navigation elements."
                                }
                            }
                            div { class: "flex bg-slate-100 p-1 rounded-lg",
                                button { class: "px-4 py-1.5 text-xs font-semibold bg-white rounded-md shadow-sm",
                                    "Default"
                                }
                                button { class: "px-4 py-1.5 text-xs font-semibold text-slate-500",
                                    "Compact"
                                }
                            }
                        }
                    }
                }
                div { class: "settings-section",
                    div { class: "flex items-center gap-3 mb-6",
                        MaterialIcon { name: "psychology", color: MaterialIconColor::Custom("text-slate-400".to_string()), size: 24 }
                        h3 { class: "text-lg font-bold text-slate-900", "AI Engine" }
                    }
                    div { class: "space-y-6",
                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-sm font-semibold text-slate-900",
                                    "Default Model"
                                }
                                p { class: "text-xs text-slate-500",
                                    "Select which local model to use for primary chat."
                                }
                            }
                            select { class: "bg-white border-slate-200 rounded-lg text-sm w-64 focus:ring-1 focus:ring-brand-blue/20 focus:border-brand-blue py-2 px-3 shadow-sm",
                                option { "Llama 3 (8B) - Active" }
                                option { "Mistral v0.3 (7B)" }
                                option { "Phi-3 Mini" }
                            }
                        }
                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-sm font-semibold text-slate-900",
                                    "Context Window"
                                }
                                p { class: "text-xs text-slate-500",
                                    "Maximum tokens available for processing."
                                }
                            }
                            div { class: "w-64",
                                input {
                                    class: "w-full h-2 bg-slate-200 rounded-lg appearance-none cursor-pointer accent-brand-blue",
                                    max: "32768",
                                    min: "1024",
                                    r#type: "range",
                                    value: "8192",
                                }
                                div { class: "flex justify-between mt-2",
                                    span { class: "text-[10px] font-bold text-slate-400",
                                        "1K"
                                    }
                                    span { class: "text-[10px] font-bold text-brand-blue",
                                        "8K Tokens"
                                    }
                                    span { class: "text-[10px] font-bold text-slate-400",
                                        "32K"
                                    }
                                }
                            }
                        }
                    }
                }
                div { class: "settings-section",
                    div { class: "flex items-center gap-3 mb-6",
                        MaterialIcon { name: "shield", color: MaterialIconColor::Custom("text-slate-400".to_string()), size: 24 }
                        h3 { class: "text-lg font-bold text-slate-900", "Privacy & Security" }
                    }
                    div { class: "space-y-6",
                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-sm font-semibold text-slate-900",
                                    "Local Only Mode"
                                }
                                p { class: "text-xs text-slate-500",
                                    "Prevent any external API calls for processing."
                                }
                            }
                            div { class: "relative inline-flex items-center cursor-pointer",
                                input {
                                    checked: "",
                                    class: "sr-only peer",
                                    r#type: "checkbox",
                                }
                                div { class: "w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-brand-blue" }
                            }
                        }
                        div { class: "flex items-center justify-between",
                            div {
                                p { class: "text-sm font-semibold text-slate-900",
                                    "Conversation History"
                                }
                                p { class: "text-xs text-slate-500",
                                    "Store chats locally on this device."
                                }
                            }
                            div { class: "relative inline-flex items-center cursor-pointer",
                                input {
                                    checked: "",
                                    class: "sr-only peer",
                                    r#type: "checkbox",
                                }
                                div { class: "w-11 h-6 bg-slate-200 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-brand-blue" }
                            }
                        }
                    }
                }
                div { class: "flex justify-end gap-3 mt-8 pb-12",
                    button { class: "px-6 py-2.5 rounded-lg text-sm font-semibold text-slate-600 hover:bg-slate-200 transition-colors",
                        "Reset Defaults"
                    }
                    button { class: "px-6 py-2.5 rounded-lg text-sm font-semibold bg-primary text-white hover:bg-slate-800 shadow-sm transition-all active:scale-[0.98]",
                        "Save Changes"
                    }
                }
                footer { class: "py-12 border-t border-slate-200",
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
}

