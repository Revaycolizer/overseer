use dioxus::prelude::*;
use dioxus::document::eval;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::routes::conversations::types::{VizKind, VizBlock};
use crate::routes::conversations::search_view::SearchBlock;
use crate::engine::tools::charting::{ChartRequest, generate_chart_svg};

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

    let mut svg_status = use_signal(|| 0); // 0=idle, 1=saving, 2=saved, 3=err
    let mut png_status = use_signal(|| 0);

    rsx! {
        div { class: "w-full space-y-2",
            div { class: "bg-white rounded-2xl border border-slate-100 shadow-sm overflow-hidden",
                if block.kind == VizKind::Data {
                    DataChart { payload: block.payload.clone() }
                } else if is_asset && html_from_file.read().is_none() {
                    div { class: "flex items-center justify-center h-[200px] text-slate-400 gap-3",
                        div { class: "size-5 border-2 border-blue-500 border-t-transparent rounded-full animate-spin" }
                        "Preparing visualization..."
                    }
                } else if block.kind == VizKind::Svg {
                    div { class: "w-full p-4 lg:p-8 dangerous-svg-container [&>svg]:w-full [&>svg]:h-auto [&>svg]:display-block",
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

                div { class: "flex items-center gap-2",
                    // Export SVG
                    button {
                        class: "flex items-center gap-1.5 px-2 py-1 text-[9px] font-semibold text-slate-500 hover:bg-slate-100 rounded-lg transition-all",
                        onclick: {
                            let svg_raw = block.payload.clone();
                            let is_svg = block.kind == VizKind::Svg;
                            move |_| {
                                let payload = svg_raw.clone();
                                svg_status.set(1);
                                let mut eval = eval(
                                    r#"
                                    let content = await (async () => {
                                        return document.querySelector('[title="Visualization"]')?.srcdoc || document.querySelector('.dangerous-svg-container')?.innerHTML || window.last_captured_svg || "";
                                    })();
                                    dioxus.send(content);
                                    "#
                                );
                                spawn(async move {
                                    if let Ok(value) = eval.recv().await {
                                        let value: serde_json::Value = value;
                                        if let Some(json) = value.as_str() {
                                            let final_data = if is_svg { payload.clone() } else { json.to_string() };
                                            let _ = crate::backend::save_chart_export("chart_export".to_string(), final_data, "svg".to_string()).await;
                                            svg_status.set(2);
                                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                            svg_status.set(0);
                                        }
                                    }
                                });
                            }
                        },
                        {
                            if svg_status() == 0 {
                                rsx! { 
                                    MaterialIcon { name: "download", size: 12, color: MaterialIconColor::Custom("text-slate-400".to_string()) }
                                    "SVG"
                                }
                            } else if svg_status() == 1 {
                                rsx! { "Saving..." }
                            } else {
                                 rsx! { "Saved! ✅" }
                            }
                        }
                    }
                    
                    // Export PNG (Capture canvas)
                    button {
                        class: "flex items-center gap-1.5 px-2 py-1 text-[9px] font-semibold text-white bg-blue-500 hover:bg-blue-600 rounded-lg transition-all shadow-sm disabled:opacity-50",
                        disabled: png_status() != 0,
                        onclick: {
                            let _svg_raw = block.payload.clone();
                            move |_| {
                                png_status.set(1);
                                let mut eval = eval(
                                    r#"
                                    async function capturePNG() {
                                        const svgContainer = document.querySelector('.dangerous-svg-container') || document.querySelector('[title="Visualization"]');
                                        if (!svgContainer) return null;
                                        
                                        const svg = svgContainer.querySelector('svg') || svgContainer;
                                        const svgString = new XMLSerializer().serializeToString(svg);
                                        const canvas = document.createElement('canvas');
                                        const ctx = canvas.getContext('2d');
                                        const img = new Image();
                                        const svgBlob = new Blob([svgString], {type: 'image/svg+xml;charset=utf-8'});
                                        const url = URL.createObjectURL(svgBlob);
                                        
                                        return new Promise((resolve) => {
                                            img.onload = () => {
                                                canvas.width = img.width * 2; // High res
                                                canvas.height = img.height * 2;
                                                ctx.fillStyle = "white";
                                                ctx.fillRect(0, 0, canvas.width, canvas.height);
                                                ctx.drawImage(img, 0, 0, canvas.width, canvas.height);
                                                URL.revokeObjectURL(url);
                                                resolve(canvas.toDataURL('image/png'));
                                            };
                                            img.src = url;
                                        });
                                    }
                                    const dataUrl = await capturePNG();
                                    dioxus.send(dataUrl);
                                    "#
                                );
                                spawn(async move {
                                    if let Ok(value) = eval.recv().await {
                                        let value: serde_json::Value = value;
                                        if let Some(data_url) = value.as_str() {
                                            let _ = crate::backend::save_chart_export("chart_export".to_string(), data_url.to_string(), "png".to_string()).await;
                                            png_status.set(2);
                                            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                            png_status.set(0);
                                        }
                                    }
                                });
                            }
                        },
                        {
                            if png_status() == 0 {
                                rsx! { 
                                    MaterialIcon { name: "image", size: 12, color: MaterialIconColor::Custom("text-white".to_string()) }
                                    "PNG"
                                }
                            } else if png_status() == 1 {
                                 rsx! { "Saving..." }
                            } else {
                                 rsx! { "Saved! ✅" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn DataChart(payload: String) -> Element {
    let req: ChartRequest = match serde_json::from_str(&payload) {
        Ok(d) => d,
        Err(_) => {
            // Fallback for slightly different schemas
            match serde_json::from_str::<serde_json::Value>(&payload) {
                Ok(v) => {
                    ChartRequest {
                        chart_type: v.get("chart_type").and_then(|v| v.as_str()).unwrap_or("bar").to_string(),
                        title: v.get("title").and_then(|v| v.as_str()).unwrap_or("Chart").to_string(),
                        labels: v.get("labels").and_then(|v| v.as_array()).map(|a| a.iter().map(|v| v.as_str().unwrap_or_default().to_string()).collect()).unwrap_or_default(),
                        data: v.get("data").and_then(|v| v.as_array()).map(|a| a.iter().map(|v| v.as_f64().unwrap_or_default()).collect()).unwrap_or_default(),
                    }
                },
                Err(e) => return rsx! { div { class: "p-4 text-red-500", "Chart Error: {e}" } }
            }
        }
    };
    
    let svg_res = generate_chart_svg(req);
    
    match svg_res {
        Ok(svg) => rsx! {
            div { class: "w-full p-4 lg:p-8 dangerous-svg-container [&>svg]:w-full [&>svg]:h-[300px] [&>svg]:display-block bg-white flex items-center justify-center",
                dangerous_inner_html: "{svg}"
            }
        },
        Err(e) => rsx! { div { class: "p-4 text-red-500", "Render Error: {e}" } }
    }
}
