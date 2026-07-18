use dioxus::prelude::*;
use crate::backend;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::engine::orchestrator::utils::json_strip::strip_embedded_json;
use crate::routes::conversations::helpers::clean_content;
use crate::routes::conversations::types::StructuredResponse;
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

    // Some small models (qwen2.5 7b in particular) wrap their structured
    // response as plain text inside `messages[].message` instead of
    // emitting the proper JSON shape. Strip any embedded JSON shell so
    // the renderer shows just the prose answer instead of
    // "answer + duplicate JSON".
    if let Some(res) = structured.as_mut() {
        for m in res.messages.iter_mut() {
            m.message = strip_embedded_json(&m.message);
        }
    }
    
    rsx! {
        div {
            class: "flex items-start gap-3 mb-5 animate-fade-in",
            style: "content-visibility: auto; contain-intrinsic-size: auto 200px;",
            div { class: "size-7 shrink-0 rounded-xl bg-blue-600 flex items-center justify-center text-white shadow-[0_2px_8px_rgba(59,130,246,0.3)] transition-all duration-200 hover:scale-110 hover:shadow-[0_4px_12px_rgba(59,130,246,0.4)]",
                MaterialIcon { name: "smart_toy", color: MaterialIconColor::Light, size: 18 }
            }
            div { class: "space-y-3 flex-1 min-w-0",
                div { class: "flex items-center gap-2",
                    span { class: "text-[11px] font-semibold uppercase tracking-[0.15em] text-slate-400", "Overseer" }
                    span { class: "size-1 rounded-full bg-slate-300" }
                    span { class: "text-[10px] text-slate-400", "just now" }
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
    let attachments = msg.attachments.clone();
    rsx! {
        div { class: "flex items-start gap-3 flex-row-reverse animate-fade-in",
            div { class: "size-7 shrink-0 rounded-xl border border-slate-200 bg-white flex items-center justify-center overflow-hidden shadow-[0_1px_3px_rgba(0,0,0,0.06)]",
                img { class: "w-full h-full object-cover", src: "https://lh3.googleusercontent.com/aida-public/AB6AXuBqMBjxuGBSeYlBYQz3ehCFqzRwa9Wyq9eVwMYy09GJ935PrL_9J9Nm026Ma8JaIJeKfVWN7hdeIFL0gi4M4lOputd5eqoppoLRMUGbZnFachbwDjykRgdxvZCEKJojPy5zMgRnDuAjxuLndEN_6WYnHU9MPtepRB2SOYmZ_WbKPgUjNyE5Z8A9NuRt-cff_feV6mAy2VkbTR38GeWrNCj20LzICJCLsrMqsI68pBlMyS4UV1ORb1_pYK5tyDwjtOlKMwPP2uuBrtw" }
            }
            div { class: "group relative w-full sm:w-auto sm:max-w-[80%] text-right",
                div { class: "bg-[var(--user-bubble)] p-3.5 rounded-2xl rounded-tr-none border border-slate-200 text-sm leading-relaxed shadow-[0_1px_4px_rgba(0,0,0,0.04)] transition-shadow duration-200 hover:shadow-[0_2px_8px_rgba(0,0,0,0.06)]",
                    p { class: "select-text", "{content}" }
                }
                if !attachments.is_empty() {
                    div { class: "mt-2 flex flex-wrap gap-2 justify-end",
                        for att in attachments {
                            AttachmentPreview { att }
                        }
                    }
                }
                button {
                    class: "absolute -left-9 top-2 p-1.5 rounded-lg opacity-0 group-hover:opacity-100 transition-all duration-200 ease-out hover:bg-slate-100 active:scale-90",
                    onclick: move |_| on_copy.call(content.clone()),
                    MaterialIcon { name: "content_copy", size: 14 }
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
        div { class: "space-y-3",
            if let Some(t) = text {
                div { class: "bg-[var(--assistant-bubble)] p-3.5 rounded-2xl rounded-tl-none border border-blue-50/40 shadow-[0_1px_4px_rgba(0,0,0,0.03)] w-full sm:max-w-[95%] transition-shadow duration-200 hover:shadow-[0_2px_8px_rgba(0,0,0,0.05)]",
                    MarkdownText { content: t, is_user: false }
                }
            }
            for block in blocks {
                ChartBlock { block, hide_evidence: msg.hide_evidence }
            }
        }
    }
}
