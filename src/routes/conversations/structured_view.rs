use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::routes::conversations::types::{StructuredResponse, SourcePart, UnknownPart, VizBlock, VizKind};
use crate::routes::conversations::markdown_renderer::MarkdownText;
use crate::routes::conversations::attachment_view::AttachmentPreview;
use crate::routes::conversations::chart_view::ChartBlock;
use crate::routes::conversations::helpers::copy_to_clipboard;
use crate::backend;

#[component]
pub fn StructuredView(res: StructuredResponse, hide_evidence: bool) -> Element {
    rsx! {
        div { class: "space-y-4",
            for (i, msg) in res.messages.iter().enumerate() {
                div { 
                    key: "msg-{i}",
                    class: "group relative bg-[var(--assistant-bubble)] p-4 rounded-2xl rounded-tl-none border border-blue-50/50 text-[var(--text-main)] text-sm leading-relaxed shadow-bubble max-w-[95%] w-fit",
                    
                    button {
                        class: "absolute top-2 right-2 p-1.5 opacity-0 group-hover:opacity-100 hover:bg-white/50 rounded-lg transition-all z-10",
                        onclick: {
                            let content = msg.message.clone();
                            move |_| copy_to_clipboard(content.clone())
                        },
                        title: "Copy message",
                        MaterialIcon { name: "content_copy", size: 14, color: MaterialIconColor::Custom("text-slate-400".to_string()) }
                    }

                    MarkdownText { content: msg.message.clone(), is_user: msg.role == "user" }
                }
            }
            if !res.sources.is_empty() && !hide_evidence {
                div { class: "max-w-[95%] space-y-2",
                    div { class: "text-[10px] font-bold text-slate-500 uppercase tracking-widest px-1", "Sources" }
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-2",
                        for src in &res.sources {
                            SourceCard { src: src.clone() }
                        }
                    }
                }
            }
            if !res.attachments.is_empty() {
                div { class: "max-w-[95%] space-y-3",
                    for path in &res.attachments {
                        AttachmentPreview { att: backend::Attachment {
                            filename: path.split('/').last().unwrap_or("file").to_string(),
                            file_path: path.clone(),
                            content_type: if path.ends_with(".png") || path.ends_with(".jpg") { "image/png" } else { "application/octet-stream" }.to_string(),
                            ..Default::default()
                        }}
                    }
                }
            }
            for (i, chart) in res.charts.iter().enumerate() {
                {
                    let kind = match chart.chart_type.to_lowercase().as_str() {
                        "svg" | "html" => VizKind::Svg,
                        "bar" | "line" | "pie" | "data" => VizKind::Data,
                        _ => VizKind::Asset,
                    };
                    
                    let mut payload = chart.value.clone().unwrap_or_default();
                    if kind == VizKind::Data && (payload.is_empty() || payload.starts_with('{')) {
                         // Serialize the whole chart part so DataChart can extract labels/data
                         if let Ok(json) = serde_json::to_string(chart) {
                             payload = json;
                         }
                    }

                    rsx! {
                        ChartBlock { 
                            key: "{i}",
                            block: VizBlock {
                                kind,
                                payload,
                            },
                            hide_evidence: hide_evidence,
                        }
                    }
                }
            }
            for unk in &res.unknown {
                UnknownRenderer { part: unk.clone() }
            }
        }
    }
}

#[component]
fn SourceCard(src: SourcePart) -> Element {
    rsx! {
        a { href: "{src.link}", target: "_blank", class: "block p-3 bg-white rounded-xl border border-slate-100 shadow-sm hover:shadow-md hover:border-blue-200 transition-all",
            div { class: "text-[10px] font-bold text-blue-600 mb-1", "{src.source}" }
            div { class: "text-[11px] text-slate-600 line-clamp-2", "{src.description}" }
        }
    }
}

#[component]
fn UnknownRenderer(part: UnknownPart) -> Element {
    match part.kind.as_str() {
        "text" => rsx! { MarkdownText { content: part.data.as_str().unwrap_or_default().to_string() } },
        "image" | "media" => {
            let path = part.data.as_str().unwrap_or_default();
            rsx! { AttachmentPreview { att: backend::Attachment {
                filename: path.split('/').last().unwrap_or("file").to_string(),
                file_path: path.to_string(),
                content_type: "image/png".to_string(), // Heuristic
                ..Default::default()
            }}}
        },
        _ => rsx! { div { class: "p-2 bg-slate-100 rounded text-[10px] text-slate-400 italic", "Unsupported content type: {part.kind}" } }
    }
}
