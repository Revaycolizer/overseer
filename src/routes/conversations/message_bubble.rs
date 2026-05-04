use dioxus::prelude::*;
use crate::backend;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::routes::conversations::helpers::clean_content;
use crate::routes::conversations::types::{StructuredResponse, VizBlock};
use crate::routes::conversations::markdown_renderer::MarkdownText;
use crate::routes::conversations::attachment_view::AttachmentPreview;
use crate::routes::conversations::chart_view::ChartBlock;
use crate::routes::conversations::structured_view::StructuredView;
use crate::routes::conversations::legacy_parser::parse_content;

#[component]
pub fn MessageBubble(msg: backend::Message, on_copy: EventHandler<String>) -> Element {
    if msg.role == "user" { return rsx! { UserBubble { msg, on_copy } }; }

    let mut structured: Option<StructuredResponse> = serde_json::from_str(&msg.content).ok();
    
    // Fallback: If it's a SupervisorExecutionResult (standard engine result), convert it for the UI
    if structured.is_none() {
        if let Ok(res) = serde_json::from_str::<crate::engine::orchestrator::SupervisorExecutionResult>(&msg.content) {
            structured = Some(crate::engine::orchestrator::utils::formatter::format_supervisor_result(&res));
        }
    }
    
    rsx! {
        div { class: "flex items-start gap-4 mb-6",
            div { class: "size-8 shrink-0 rounded-lg bg-blue-600 flex items-center justify-center text-white shadow-soft transition-transform hover:scale-105",
                MaterialIcon { name: "smart_toy", color: MaterialIconColor::Light, size: 20 }
            }
            div { class: "space-y-4 flex-1 min-w-0",
                div { class: "flex items-center gap-2",
                    span { class: "text-[11px] font-bold uppercase tracking-widest text-slate-500", "Overseer" }
                }
                if let Some(res) = structured {
                    StructuredView { res, hide_evidence: msg.hide_evidence }
                } else {
                    LegacyAssistantBubble { msg, on_copy }
                }
            }
        }
    }
}

#[component]
fn UserBubble(msg: backend::Message, on_copy: EventHandler<String>) -> Element {
    let content = msg.content.clone();
    rsx! {
        div { class: "flex items-start gap-4 flex-row-reverse",
            div { class: "size-8 shrink-0 rounded-lg border border-slate-200 bg-white flex items-center justify-center overflow-hidden shadow-soft",
                img { class: "w-full h-full object-cover", src: "https://lh3.googleusercontent.com/aida-public/AB6AXuBqMBjxuGBSeYlBYQz3ehCFqzRwa9Wyq9eVwMYy09GJ935PrL_9J9Nm026Ma8JaIJeKfVWN7hdeIFL0gi4M4lOputd5eqoppoLRMUGbZnFachbwDjykRgdxvZCEKJojPy5zMgRnDuAjxuLndEN_6WYnHU9MPtepRB2SOYmZ_WbKPgUjNyE5Z8A9NuRt-cff_feV6mAy2VkbTR38GeWrNCj20LzICJCLsrMqsI68pBlMyS4UV1ORb1_pYK5tyDwjtOlKMwPP2uuBrtw" }
            }
            div { class: "group relative max-w-[85%] text-right",
                div { class: "bg-[var(--user-bubble)] p-4 rounded-2xl rounded-tl-none border border-slate-200 text-sm shadow-bubble",
                    p { class: "select-text", "{content}" }
                }
                button { class: "absolute -left-10 top-2 p-1.5 opacity-0 group-hover:opacity-100 transition-all shadow-sm", onclick: move |_| on_copy.call(content.clone()),
                    MaterialIcon { name: "content_copy", size: 16 }
                }
            }
        }
    }
}

#[component]
fn LegacyAssistantBubble(msg: backend::Message, on_copy: EventHandler<String>) -> Element {
    let raw = clean_content(&msg.content);
    let (text, blocks) = parse_content(&raw);
    rsx! {
        div { class: "space-y-4",
            if let Some(t) = text {
                div { class: "bg-[var(--assistant-bubble)] p-4 rounded-2xl rounded-tl-none border border-blue-50/50 shadow-bubble max-w-[95%] w-fit",
                    MarkdownText { content: t, is_user: false }
                }
            }
            for block in blocks {
                ChartBlock { block, hide_evidence: msg.hide_evidence }
            }
        }
    }
}
