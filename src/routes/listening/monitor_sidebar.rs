use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn MonitorSidebar() -> Element {
    rsx! {
        aside { class: "w-80 border-l border-outline-variant/10 bg-white flex flex-col shrink-0 overflow-hidden",
            div { class: "p-6 border-b border-outline-variant/10",
                div { class: "flex items-center justify-between mb-6",
                    span { class: "text-[10px] font-bold uppercase tracking-widest text-on-surface-variant", "System Load" }
                    MaterialIcon { name: "monitoring", color: MaterialIconColor::Custom("text-secondary".into()) }
                }
                div { class: "space-y-6",
                    SystemLoadBars {}
                    GpuPerformanceCard {}
                }
            }
            div { class: "flex-1 overflow-y-auto custom-scrollbar p-6",
                h3 { class: "text-[10px] font-bold uppercase tracking-widest text-on-surface-variant mb-4", "Background Processes" }
                div { class: "space-y-3",
                    TaskCard { title: "Database Indexing", sub: "Local Assets", progress: 88, icon: "database" }
                    TaskCard { title: "Cloud Sync", sub: "Awaiting Network", progress: 0, icon: "cloud_sync", is_paused: true }
                    TaskCard { title: "Audit Service", sub: "Idle", progress: 0, icon: "security", is_idle: true }
                }
            }
            FooterContext {}
        }
    }
}

#[component]
fn SystemLoadBars() -> Element {
    rsx! {
        div { class: "grid grid-cols-2 gap-4",
            div { class: "bg-surface rounded-lg p-3 border border-outline-variant/5",
                div { class: "flex justify-between items-center mb-1",
                    span { class: "text-[10px] font-bold text-on-surface-variant", "CPU" }
                    span { class: "text-xs font-bold text-secondary", "24%" }
                }
                div { class: "w-full h-1.5 bg-surface-container rounded-full overflow-hidden",
                    div { class: "h-full bg-secondary w-[24%]" }
                }
            }
            div { class: "bg-surface rounded-lg p-3 border border-outline-variant/5",
                div { class: "flex justify-between items-center mb-1",
                    span { class: "text-[10px] font-bold text-on-surface-variant", "RAM" }
                    span { class: "text-xs font-bold text-secondary", "4.2GB" }
                }
                div { class: "w-full h-1.5 bg-surface-container rounded-full overflow-hidden",
                    div { class: "h-full bg-secondary w-[42%]" }
                }
            }
        }
    }
}

#[component]
fn GpuPerformanceCard() -> Element {
    rsx! {
        div { class: "bg-slate-900 rounded-xl p-4 text-white",
            div { class: "flex justify-between items-center mb-3",
                span { class: "text-[10px] font-bold uppercase tracking-widest opacity-60", "GPU Performance" }
                span { class: "text-xs font-bold text-blue-400", "92% Load" }
            }
            div { class: "flex items-end gap-1 h-12 mb-2",
                div { class: "w-full bg-blue-600/30 h-[20%] rounded-t-sm" }
                div { class: "w-full bg-blue-600/30 h-[40%] rounded-t-sm" }
                div { class: "w-full bg-blue-600/30 h-[30%] rounded-t-sm" }
                div { class: "w-full bg-blue-600/30 h-[60%] rounded-t-sm" }
                div { class: "w-full bg-blue-600 h-[80%] rounded-t-sm" }
                div { class: "w-full bg-blue-400 h-[92%] rounded-t-sm" }
            }
            span { class: "text-[10px] font-medium opacity-50 block text-center", "NVIDIA RTX 4090 Synthesis Unit" }
        }
    }
}

#[component]
fn TaskCard(title: &'static str, sub: &'static str, progress: u8, icon: &'static str, is_paused: Option<bool>, is_idle: Option<bool>) -> Element {
    let idle_class = if is_idle.unwrap_or(false) { "opacity-60" } else { "" };
    
    let icon_color = if progress > 0 { "text-secondary" } else { "text-on-surface-variant" };

    rsx! {
        div { class: "bg-surface border border-outline-variant/10 rounded-lg p-3 {idle_class}",
            div { class: "flex items-start justify-between mb-3",
                div { class: "flex items-center gap-2",
                    div { class: "w-8 h-8 rounded-lg bg-white flex items-center justify-center shadow-sm",
                        MaterialIcon { name: icon, size: 14, color: MaterialIconColor::Custom(icon_color.into()) }
                    }
                    div {
                        h4 { class: "text-xs font-bold", "{title}" }
                        span { class: "text-[9px] text-on-surface-variant uppercase", "{sub}" }
                    }
                }
                if let Some(true) = is_paused {
                    button { class: "text-on-surface-variant hover:text-secondary",
                        MaterialIcon { name: "pause_circle", size: 18 }
                    }
                } else if !is_idle.unwrap_or(false) {
                    span { class: "text-xs font-bold text-secondary", "{progress}%" }
                }
            }
            if progress > 0 {
                div { class: "w-full h-1 bg-surface-container-high rounded-full overflow-hidden",
                    div { class: "h-full bg-secondary w-[{progress}%]" }
                }
            }
        }
    }
}

#[component]
fn FooterContext() -> Element {
    rsx! {
        div { class: "p-6 bg-slate-50 border-t border-outline-variant/10",
            div { class: "rounded-lg overflow-hidden relative h-32 group cursor-pointer shadow-sm border border-outline-variant/10",
                img {
                    alt: "Server room",
                    class: "w-full h-full object-cover grayscale brightness-50",
                    src: "https://lh3.googleusercontent.com/aida-public/AB6AXuBMArK0diiTwzPt7V8XzHgloGrLu1qYBNwTAVE4pJ9IP_h0UaXZbQo2XqXXPbXFJa6OkvK_D1gnVu5tdBUWnPYN1Fao_0-7HmY1CvGPTSEJI9fpf8wbcSbisEfrZ-m3lM47dfNaRU8ZIV1axuLUwC5DTrAZc3q_sbPnzPcnToBwZQe5VMC5kzEI0_Px-NXx9SLAx1L1kpg8CYBXvfPgb-sT0MuA7wZzx3gM_TZ_7u_uzvN5sgwUlfl2F-gzfMge0I-3mEGnY2fIPmg"
                }
                div { class: "absolute inset-0 flex flex-col justify-end p-3",
                    span { class: "text-[8px] uppercase font-bold text-white/50 tracking-widest", "Environment" }
                    div { class: "flex items-center justify-between",
                        span { class: "text-[11px] font-bold text-white", "Foundry Node-01" }
                        span { class: "text-[9px] font-bold text-green-400 flex items-center gap-1",
                            span { class: "w-1.5 h-1.5 bg-green-400 rounded-full" }
                            "Online"
                        }
                    }
                }
            }
        }
    }
}
