use super::helpers::{clean_content, copy_to_clipboard};
use crate::backend;
use base64::{engine::general_purpose, Engine as _};
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use serde::Deserialize;

// ── A single visualization block extracted from message content ───────────────
#[derive(Clone, Debug, PartialEq)]
struct VizBlock {
    kind: VizKind,
    payload: String,
}

#[derive(Clone, Debug, PartialEq)]
enum VizKind {
    Svg,
    Html,
    Asset,
    Search,
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
    image_url: Option<String>,
}

/// Parse the cleaned content into an optional leading text block
/// and zero or more visualization blocks.
fn parse_content(content: &str) -> (Option<String>, Vec<VizBlock>) {
    let markers = [
        ("---CHART_SVG---", VizKind::Svg),
        ("---CHART_HTML---", VizKind::Html),
        ("---CHART_ASSET---", VizKind::Asset),
        ("---SEARCH_RESULTS---", VizKind::Search),
    ];

    let mut text_part: Option<String> = None;
    let mut viz_blocks: Vec<VizBlock> = Vec::new();
    let mut remaining = content;

    // Find first marker position to split off leading text
    let first_marker_pos = markers.iter().filter_map(|(m, _)| remaining.find(m)).min();

    if let Some(pos) = first_marker_pos {
        let before = remaining[..pos].trim();
        if !before.is_empty() {
            text_part = Some(before.to_string());
        }
        remaining = &remaining[pos..];
    } else {
        // No markers at all — pure text
        let t = content.trim();
        if !t.is_empty() {
            text_part = Some(t.to_string());
        }
        return (text_part, viz_blocks);
    }

    // Now extract all visualization blocks from remaining
    while !remaining.is_empty() {
        let mut found = false;
        for (marker, kind) in &markers {
            if remaining.starts_with(marker) {
                let after = &remaining[marker.len()..];
                // Find where this block ends (next marker or end of string)
                let next_marker_pos = markers
                    .iter()
                    .filter_map(|(m, _)| if *m == *marker { None } else { after.find(m) })
                    .chain(markers.iter().filter_map(|(m, _)| {
                        if *m == *marker {
                            // Also check same marker again
                            after.find(m)
                        } else {
                            None
                        }
                    }))
                    .min();

                // Also check for same marker appearing again
                let same_marker_pos = after.find(marker);
                let end_pos = match (next_marker_pos, same_marker_pos) {
                    (Some(a), Some(b)) => Some(a.min(b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };

                let payload = match end_pos {
                    Some(end) => after[..end].trim().to_string(),
                    None => after.trim().to_string(),
                };

                viz_blocks.push(VizBlock {
                    kind: kind.clone(),
                    payload,
                });

                remaining = match end_pos {
                    Some(end) => &after[end..],
                    None => "",
                };
                found = true;
                break;
            }
        }
        if !found {
            // Skip non-marker content between blocks
            let next_pos = markers.iter().filter_map(|(m, _)| remaining.find(m)).min();
            match next_pos {
                Some(pos) => remaining = &remaining[pos..],
                None => break,
            }
        }
    }

    (text_part, viz_blocks)
}

#[component]
fn SearchBlock(payload: String, hide_evidence: bool) -> Element {
    if hide_evidence {
        return rsx! {};
    }
    let results = serde_json::from_str::<Vec<SearchResult>>(&payload).unwrap_or_default();

    if results.is_empty() {
        return rsx! {
            div { class: "p-4 text-slate-400 text-xs italic bg-slate-50 rounded-xl border border-slate-100",
                "No structured results found."
            }
        };
    }

    rsx! {
        div { class: "w-full space-y-3",
            // Header
            div { class: "flex items-center gap-2 px-1",
                MaterialIcon {
                    name: "travel_explore",
                    size: 16,
                    color: MaterialIconColor::Custom("text-blue-500".to_string())
                }
                span { class: "text-[10px] font-bold text-slate-500 uppercase tracking-widest",
                    "Intelligence Report"
                }
            }

            // Results Grid/List
            div { class: "grid grid-cols-1 md:grid-cols-2 gap-3",
                for res in results {
                    a {
                        href: "{res.url}",
                        target: "_blank",
                        class: "group flex flex-col bg-white rounded-xl border border-slate-100 shadow-sm hover:shadow-md hover:border-blue-200 transition-all overflow-hidden",

                        // Image/Icon Area
                        div { class: "relative h-32 bg-slate-50 flex items-center justify-center overflow-hidden border-b border-slate-50",
                            if let Some(img) = res.image_url {
                                img {
                                    src: "{img}",
                                    class: "w-full h-full object-cover transition-transform group-hover:scale-105",
                                    onerror: move |_| {
                                        // Silent fail - will show placeholder if icon fails
                                    }
                                }
                            } else {
                                MaterialIcon {
                                    name: "public",
                                    size: 48,
                                    color: MaterialIconColor::Custom("text-slate-200".to_string())
                                }
                            }

                            // Domain badge
                            div { class: "absolute bottom-2 left-2 px-2 py-0.5 bg-black/60 backdrop-blur-sm rounded text-[9px] text-white font-medium",
                                {res.url.split('/').nth(2).unwrap_or("Link")}
                            }
                        }

                        // Content
                        div { class: "p-3 space-y-1.5",
                            h3 { class: "text-xs font-bold text-slate-800 line-clamp-2 group-hover:text-blue-600 transition-colors",
                                "{res.title}"
                            }
                            p { class: "text-[10px] text-slate-500 line-clamp-3 leading-relaxed",
                                "{res.snippet}"
                            }
                        }
                    }
                }
            }

            div { class: "text-[9px] text-slate-300 font-medium text-right px-1",
                "Aggregated via Stealth Research"
            }
        }
    }
}

// ── Single chart block renderer ───────────────────────────────────────────────
#[component]
fn MarkdownText(text: String) -> Element {
    let segments = use_memo(move || parse_markdown_text(&text));

    rsx! {
        span { class: "select-text whitespace-pre-wrap",
            for seg in segments() {
                match seg {
                    MarkdownSegment::Plain(t) => rsx! { "{t}" },
                    MarkdownSegment::Bold(t) => rsx! { b { class: "font-bold", "{t}" } },
                    MarkdownSegment::Code(t) => rsx! {
                        pre { class: "my-2 p-3 bg-slate-950 text-slate-100 text-[11px] font-mono rounded-lg overflow-x-auto border border-slate-800 shadow-inner",
                            code { "{t}" }
                        }
                    },
                    MarkdownSegment::Link { title, url } => rsx! {
                        a {
                            href: "{url}",
                            target: "_blank",
                            class: "text-blue-500 hover:text-blue-700 underline font-medium break-all transition-colors",
                            "{title}"
                        }
                    },
                    MarkdownSegment::Image { alt, url } => rsx! {
                        img {
                            src: "{url}",
                            alt: "{alt}",
                            class: "my-4 rounded-xl border border-slate-200 shadow-sm max-w-full h-auto block mx-auto",
                            onerror: move |_| {
                                // Silent fail - could add a fallback icon later if needed
                            }
                        }
                    },
                }
            }
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
enum MarkdownSegment {
    Plain(String),
    Bold(String),
    Code(String),
    Link { title: String, url: String },
    Image { alt: String, url: String },
}

fn parse_markdown_text(text: &str) -> Vec<MarkdownSegment> {
    let mut result = Vec::new();
    let mut remaining = text;

    while !remaining.is_empty() {
        let next_code = remaining.find("```");
        let next_bold = remaining.find("**");
        let next_link = remaining.find("[");

        // Priority: Code > Bold > Link/Image
        let mut positions = vec![];
        if let Some(p) = next_code {
            positions.push((p, 0));
        }
        if let Some(p) = next_bold {
            positions.push((p, 1));
        }
        if let Some(p) = next_link {
            positions.push((p, 2));
        }
        positions.sort_by_key(|&(p, _)| p);

        if let Some(&(pos, kind)) = positions.first() {
            if pos > 0 {
                result.push(MarkdownSegment::Plain(remaining[..pos].to_string()));
            }
            let after = &remaining[pos..];

            match kind {
                0 => {
                    // Code block
                    let content = &after[3..];
                    if let Some(end) = content.find("```") {
                        result.push(MarkdownSegment::Code(content[..end].to_string()));
                        remaining = &content[end + 3..];
                    } else {
                        result.push(MarkdownSegment::Plain(after.to_string()));
                        remaining = "";
                    }
                }
                1 => {
                    // Bold
                    let content = &after[2..];
                    if let Some(end) = content.find("**") {
                        result.push(MarkdownSegment::Bold(content[..end].to_string()));
                        remaining = &content[end + 2..];
                    } else {
                        result.push(MarkdownSegment::Plain(after[..2].to_string()));
                        remaining = &after[2..];
                    }
                }
                2 => {
                    // Link or Image
                    let is_image = pos > 0 && remaining.as_bytes()[pos - 1] == b'!';
                    if is_image {
                        // Correct the preceding plain text if it was pushed with the '!'
                        if let Some(MarkdownSegment::Plain(ref mut last)) = result.last_mut() {
                            if last.ends_with('!') {
                                last.pop();
                                if last.is_empty() {
                                    result.pop();
                                }
                            }
                        }
                    }

                    let after_l = &after[1..];
                    if let Some(end_title) = after_l.find("](") {
                        let title = &after_l[..end_title];
                        let after_title = &after_l[end_title + 2..];
                        if let Some(end_url) = after_title.find(")") {
                            let url = &after_title[..end_url];
                            if is_image {
                                result.push(MarkdownSegment::Image {
                                    alt: title.to_string(),
                                    url: url.to_string(),
                                });
                            } else {
                                result.push(MarkdownSegment::Link {
                                    title: title.to_string(),
                                    url: url.to_string(),
                                });
                            }
                            remaining = &after_title[end_url + 1..];
                            continue;
                        }
                    }
                    result.push(MarkdownSegment::Plain(after[..1].to_string()));
                    remaining = &after[1..];
                }
                _ => unreachable!(),
            }
        } else {
            result.push(MarkdownSegment::Plain(remaining.to_string()));
            break;
        }
    }
    result
}

#[component]
fn AttachmentPreview(att: backend::Attachment) -> Element {
    let file_path = att.file_path.clone();
    let content_type = att.content_type.clone();
    let filename = att.filename.clone();

    let file_path_clone = file_path.clone();
    let content_type_clone = content_type.clone();
    let is_asset = file_path.starts_with("/assets/");
    let asset_data = if is_asset {
        use_resource(move || {
            let file_path = file_path_clone.clone();
            async move { Some(file_path) }
        })
    } else {
        use_resource(move || {
            let file_path = file_path_clone.clone();
            let content_type = content_type_clone.clone();
            async move {
                match std::fs::read(&file_path) {
                    Ok(bytes) if !bytes.is_empty() => Some(format!(
                        "data:{};base64,{}",
                        content_type,
                        general_purpose::STANDARD.encode(bytes)
                    )),
                    _ => None,
                }
            }
        })
    };

    let is_image = content_type.starts_with("image/");
    let icon_name = if is_image {
        "image"
    } else if content_type.contains("pdf") {
        "picture_as_pdf"
    } else if content_type.contains("spreadsheet")
        || content_type.contains("excel")
        || filename.ends_with(".xlsx")
        || filename.ends_with(".xls")
        || filename.ends_with(".csv")
    {
        "table_view"
    } else if content_type.contains("word")
        || filename.ends_with(".docx")
        || filename.ends_with(".doc")
    {
        "description"
    } else {
        "insert_drive_file"
    };

    let icon_color = if icon_name == "picture_as_pdf" {
        "text-red-500"
    } else if icon_name == "table_view" {
        "text-emerald-600"
    } else if icon_name == "description" {
        "text-blue-600"
    } else {
        "text-slate-400"
    };

    match asset_data().flatten() {
        Some(src) => {
            if is_image {
                rsx! {
                    div { class: "space-y-2",
                        a {
                            href: "{src}",
                            target: "_blank",
                            class: "block group w-fit",
                            img {
                                src: "{src}",
                                alt: "{filename}",
                                class: "rounded-2xl border border-slate-200 shadow-sm max-w-full h-auto block bg-white"
                            }
                        }
                        div { class: "text-[10px] font-semibold text-slate-500 px-1 break-all",
                            "{filename}"
                        }
                    }
                }
            } else {
                rsx! {
                    a {
                        href: "{src}",
                        download: "{filename}",
                        class: "inline-flex items-center gap-3 rounded-xl border border-slate-200 bg-white px-4 py-3 text-[12px] font-bold text-slate-700 shadow-sm hover:shadow-md hover:border-blue-200 hover:bg-blue-50/30 transition-all group",
                        div { class: "size-8 rounded-lg bg-slate-50 flex items-center justify-center group-hover:bg-white transition-colors",
                            MaterialIcon {
                                name: "{icon_name}",
                                size: 20,
                                color: MaterialIconColor::Custom(icon_color.to_string())
                            }
                        }
                        div { class: "flex flex-col gap-0.5",
                            span { class: "line-clamp-1", "{filename}" }
                            span { class: "text-[9px] text-slate-400 uppercase tracking-tighter font-black",
                                "Click to Download"
                            }
                        }
                    }
                }
            }
        }
        None => rsx! {
            div { class: "space-y-2",
                div { class: "rounded-2xl border border-slate-200 bg-white px-4 py-6 text-sm text-slate-500 shadow-sm",
                    "File preview unavailable."
                }
                div { class: "text-[10px] font-semibold text-slate-500 px-1 break-all",
                    "{filename}"
                }
            }
        },
    }
}

#[component]
fn ChartBlock(block: VizBlock, hide_evidence: bool) -> Element {
    if block.kind == VizKind::Search {
        return rsx! { SearchBlock { payload: block.payload, hide_evidence: hide_evidence } };
    }

    let is_asset = block.kind == VizKind::Asset;
    let payload = block.payload.clone();

    let html_from_file = use_resource(move || {
        let p = payload.clone();
        async move {
            if p.is_empty() {
                return None;
            }
            let clean = p
                .trim_start_matches("file:///")
                .trim_start_matches("file://")
                .to_string();
            let cwd = std::env::current_dir().unwrap_or_default();
            let abs = cwd.join(clean.trim_start_matches('/'));
            match tokio::fs::read_to_string(&abs).await {
                Ok(html) => Some(html),
                Err(e) => Some(format!(
                    "<p style='color:red;font-family:sans-serif'>Chart load error: {e}</p>"
                )),
            }
        }
    });

    let iframe_html: Option<String> = match block.kind {
        VizKind::Asset => match html_from_file() {
            Some(Some(h)) => Some(h),
            _ => None,
        },
        VizKind::Html => Some(block.payload.clone()),
        VizKind::Svg | VizKind::Search => None,
    };

    let still_loading = is_asset && matches!(html_from_file(), None);

    rsx! {
        div { class: "w-full space-y-2",
            div { class: "bg-white rounded-2xl border border-slate-100 shadow-sm overflow-hidden",
                if still_loading {
                    div { class: "flex items-center justify-center h-[200px] text-slate-400 gap-3",
                        div { class: "size-5 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" }
                        "Preparing visualization..."
                    }
                } else if block.kind == VizKind::Svg {
                    // SVG rendered inline — viewBox makes it fully responsive
                    div {
                        class: "w-full p-2 [&>svg]:w-full [&>svg]:h-auto [&>svg]:display-block",
                        dangerous_inner_html: "{block.payload}"
                    }
                } else if let Some(html) = iframe_html {
                    iframe {
                        srcdoc: "{html}",
                        class: "w-full h-[500px] border-none",
                        title: "Visualization",
                        "loading": "lazy",
                        "sandbox": "allow-scripts",
                    }
                } else if is_asset {
                    div { class: "flex items-center justify-center h-[200px] text-red-400 gap-3",
                        "Failed to load chart asset."
                    }
                }
            }
            div { class: "flex items-center justify-between px-2",
                div { class: "flex items-center gap-2",
                    MaterialIcon {
                        name: "query_stats",
                        size: 14,
                        color: MaterialIconColor::Custom("text-slate-400".to_string())
                    }
                    span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest",
                        "Dynamic Insight"
                    }
                }
                span { class: "text-[9px] text-slate-300 font-medium",
                    "Rendering via Overseer Engine"
                }
            }
        }
    }
}

// ── Main MessageBubble ────────────────────────────────────────────────────────
#[component]
pub fn MessageBubble(msg: backend::Message, on_copy: EventHandler<String>) -> Element {
    let raw_content = clean_content(&msg.content);
    let (display_text, viz_blocks) = parse_content(&raw_content);

    // ── USER BUBBLE ───────────────────────────────────────────────────────────
    if msg.role == "user" {
        return rsx! {
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
                    div { class: "group relative inline-block text-left max-w-[85%]",
                        div { class: "bg-[var(--user-bubble)] p-4 rounded-2xl rounded-tl-none text-[var(--text-main)] text-sm leading-relaxed shadow-bubble border border-slate-200",
                            p { class: "select-text", "{raw_content}" }
                        }
                        button {
                            class: "absolute -left-10 top-2 p-1.5 bg-white border border-slate-200 rounded-lg text-slate-400 hover:text-blue-600 opacity-0 group-hover:opacity-100 transition-all shadow-sm",
                            title: "Copy to clipboard",
                            onclick: move |_| on_copy.call(raw_content.clone()),
                            MaterialIcon { name: "content_copy", size: 16 }
                        }
                        if !msg.attachments.is_empty() {
                            div { class: "mt-3 flex flex-wrap gap-2 justify-end",
                                for att in &msg.attachments {
                                    div { class: "flex items-center gap-2 bg-blue-50 px-2 py-1 rounded-lg border border-blue-100 text-[10px] font-bold text-blue-600",
                                        MaterialIcon { name: "insert_drive_file", size: 14 }
                                        "{att.filename}"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        };
    }

    // ── ASSISTANT BUBBLE ──────────────────────────────────────────────────────
    rsx! {
        div { class: "flex items-start gap-4 mb-6",
            div { class: "size-8 shrink-0 rounded-lg bg-blue-600 flex items-center justify-center text-white shadow-soft transition-transform hover:scale-105",
                MaterialIcon { name: "smart_toy", color: MaterialIconColor::Light, size: 20 }
            }
            div { class: "space-y-4 flex-1 min-w-0",
                // Header
                div { class: "flex items-center gap-2",
                    span { class: "text-[11px] font-bold uppercase tracking-widest text-slate-500",
                        "Overseer"
                    }
                    span { class: "text-[9px] bg-blue-50 text-blue-600 border border-blue-100 px-1.5 py-0.5 rounded font-bold uppercase tracking-tighter",
                        "SECURE"
                    }
                }

                // Optional leading text bubble
                if let Some(text) = display_text {
                    div { class: "group relative inline-block max-w-[95%] w-fit",
                        div { class: "bg-[var(--assistant-bubble)] p-4 rounded-2xl rounded-tl-none border border-blue-50/50 text-[var(--text-main)] text-sm leading-relaxed shadow-bubble",
                            MarkdownText { text }
                        }
                        button {
                            class: "absolute -right-10 top-2 p-1.5 bg-white border border-slate-200 rounded-lg text-slate-400 hover:text-blue-600 opacity-0 group-hover:opacity-100 transition-all shadow-sm",
                            title: "Copy content",
                            onclick: move |_| on_copy.call(msg.content.clone()),
                            MaterialIcon { name: "content_copy", size: 16 }
                        }
                    }
                }

                // Render each chart block — supports multiple charts in one message
                if !msg.attachments.is_empty() {
                    div { class: "max-w-[95%] space-y-3",
                        for att in &msg.attachments {
                            AttachmentPreview { att: att.clone() }
                        }
                    }
                }
                for block in viz_blocks {
                    ChartBlock { block, hide_evidence: msg.hide_evidence }
                }
            }
        }
    }
}

#[component]
pub fn ThinkingBubble() -> Element {
    rsx! {
        div { class: "flex items-start gap-4 animate-in fade-in slide-in-from-bottom-2 duration-300",
            div { class: "size-8 shrink-0 rounded-lg bg-blue-600 flex items-center justify-center text-white shadow-soft",
                MaterialIcon { name: "smart_toy", color: MaterialIconColor::Light, size: 20 }
            }
            div { class: "space-y-2 flex-1",
                div { class: "flex items-center gap-2",
                    span { class: "text-[11px] font-bold uppercase tracking-widest text-slate-500",
                        "Overseer"
                    }
                    span { class: "text-[9px] bg-amber-50 text-amber-600 border border-amber-200 px-1.5 py-0.5 rounded font-bold uppercase tracking-tighter animate-pulse",
                        "THINKING"
                    }
                }
                div { class: "bg-[var(--assistant-bubble)] p-4 rounded-2xl rounded-tl-none border border-blue-50/50 text-[var(--text-main)] text-sm leading-relaxed shadow-bubble",
                    div { class: "flex items-center gap-2",
                        div { class: "flex gap-1",
                            span { class: "size-2 bg-blue-400 rounded-full animate-bounce", style: "animation-delay: 0ms;" }
                            span { class: "size-2 bg-blue-400 rounded-full animate-bounce", style: "animation-delay: 150ms;" }
                            span { class: "size-2 bg-blue-400 rounded-full animate-bounce", style: "animation-delay: 300ms;" }
                        }
                        span { class: "text-slate-400 text-xs ml-2",
                            "Analyzing and routing..."
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_parsing_link() {
        let text = "Check [this](https://google.com) link.";
        let segments = parse_markdown_text(text);
        assert_eq!(segments.len(), 3);
        assert!(matches!(segments[0], MarkdownSegment::Plain(ref t) if t == "Check "));
        assert!(
            matches!(segments[1], MarkdownSegment::Link { ref title, ref url } if title == "this" && url == "https://google.com")
        );
        assert!(matches!(segments[2], MarkdownSegment::Plain(ref t) if t == " link."));
    }

    #[test]
    fn test_markdown_parsing_image() {
        let text = "Look at ![this](https://example.com/img.png) image.";
        let segments = parse_markdown_text(text);
        assert_eq!(segments.len(), 3);
        assert!(matches!(segments[0], MarkdownSegment::Plain(ref t) if t == "Look at "));
        assert!(
            matches!(segments[1], MarkdownSegment::Image { ref alt, ref url } if alt == "this" && url == "https://example.com/img.png")
        );
        assert!(matches!(segments[2], MarkdownSegment::Plain(ref t) if t == " image."));
    }

    #[test]
    fn test_markdown_parsing_mixed() {
        let text = "![img](url) and [link](url)";
        let segments = parse_markdown_text(text);
        assert_eq!(segments.len(), 3);
        assert!(matches!(segments[0], MarkdownSegment::Image { .. }));
        assert!(matches!(segments[1], MarkdownSegment::Plain(ref t) if t == " and "));
        assert!(matches!(segments[2], MarkdownSegment::Link { .. }));
    }
}
