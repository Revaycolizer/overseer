use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::routes::conversations::types::{VizKind, VizBlock};
use crate::routes::conversations::search_view::SearchBlock;

#[component]
pub fn ChartBlock(block: VizBlock, hide_evidence: bool) -> Element {
    if block.kind == VizKind::Search {
        return rsx! { SearchBlock { payload: block.payload, hide_evidence: hide_evidence } };
    }

    let payload = block.payload.clone();
    let is_asset = block.kind == VizKind::Asset;

    let html_from_file = use_resource(move || {
        let p = payload.clone();
        async move {
            if p.is_empty() { return None; }
            let clean = p.trim_start_matches("file:///").trim_start_matches("file://").to_string();
            let abs = std::env::current_dir().unwrap_or_default().join(clean.trim_start_matches('/'));
            match tokio::fs::read_to_string(&abs).await {
                Ok(html) => Some(html),
                Err(e) => Some(format!("<p style='color:red;font-family:sans-serif'>Chart load error: {e}</p>")),
            }
        }
    });

    let iframe_html = match block.kind {
        VizKind::Asset => html_from_file.read().as_ref().and_then(|x| x.clone()),
        VizKind::Html => Some(block.payload.clone()),
        _ => None,
    };

    rsx! {
        div { class: "w-full space-y-2",
            div { class: "bg-white rounded-2xl border border-slate-100 shadow-sm overflow-hidden",
                if is_asset && html_from_file.read().is_none() {
                    div { class: "flex items-center justify-center h-[200px] text-slate-400 gap-3",
                        div { class: "size-5 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" }
                        "Preparing visualization..."
                    }
                } else if block.kind == VizKind::Svg {
                    div { class: "w-full p-2 [&>svg]:w-full [&>svg]:h-auto [&>svg]:display-block",
                        dangerous_inner_html: "{block.payload}"
                    }
                } else if let Some(html) = iframe_html {
                    iframe { srcdoc: "{html}", class: "w-full h-[500px] border-none", title: "Visualization", "loading": "lazy", "sandbox": "allow-scripts" }
                } else if is_asset {
                    div { class: "flex items-center justify-center h-[200px] text-red-400 gap-3", "Failed to load chart asset." }
                }
            }
            div { class: "flex items-center justify-between px-2",
                div { class: "flex items-center gap-2",
                    MaterialIcon { name: "query_stats", size: 14, color: MaterialIconColor::Custom("text-slate-400".to_string()) }
                    span { class: "text-[10px] font-bold text-slate-400 uppercase tracking-widest", "Dynamic Insight" }
                }
            }
        }
    }
}
