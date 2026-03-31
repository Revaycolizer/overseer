use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn ThinkingBubble() -> Element {
    rsx! {
        div { class: "flex items-start gap-4 mb-6 animate-in fade-in slide-in-from-bottom-2 duration-300",
            div { class: "size-8 shrink-0 rounded-lg bg-blue-600 flex items-center justify-center text-white shadow-soft transition-transform",
                MaterialIcon { name: "smart_toy", color: MaterialIconColor::Light, size: 20 }
            }
            div { class: "space-y-2 flex-1",
                div { class: "flex items-center gap-2",
                    span { class: "text-[11px] font-bold uppercase tracking-widest text-slate-500", "Overseer" }
                    span { class: "text-[9px] bg-amber-50 text-amber-600 border border-amber-200 px-1.5 py-0.5 rounded font-bold uppercase tracking-tighter animate-pulse", "THINKING" }
                }
                div { class: "bg-[var(--assistant-bubble)] p-4 rounded-2xl rounded-tl-none border border-blue-50/50 text-[var(--text-main)] text-sm leading-relaxed shadow-bubble",
                    div { class: "flex items-center gap-2",
                        div { class: "flex gap-1",
                            span { class: "size-2 bg-blue-400 rounded-full animate-bounce", style: "animation-delay: 0ms;" }
                            span { class: "size-2 bg-blue-400 rounded-full animate-bounce", style: "animation-delay: 150ms;" }
                            span { class: "size-2 bg-blue-400 rounded-full animate-bounce", style: "animation-delay: 300ms;" }
                        }
                        span { class: "text-slate-400 text-xs ml-2", "Analyzing and routing..." }
                    }
                }
            }
        }
    }
}
