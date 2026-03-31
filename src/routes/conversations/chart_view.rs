use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::routes::conversations::types::{VizKind, VizBlock};
use crate::routes::conversations::search_view::SearchBlock;

#[component]
pub fn ChartBlock(block: VizBlock, hide_evidence: bool) -> Element {
    if block.kind == VizKind::Search {
        return rsx! { SearchBlock { payload: block.payload, hide_evidence: hide_evidence } };
    }

    if block.kind == VizKind::Data {
        return rsx! { DataChart { payload: block.payload } };
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
                    div { class: "w-full p-4 [&>svg]:w-full [&>svg]:h-auto [&>svg]:display-block",
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

#[derive(serde::Deserialize, Debug, Clone, PartialEq)]
struct ChartData {
    chart_type: String,
    title: Option<String>,
    labels: Vec<String>,
    data: Vec<f64>,
}

#[component]
fn DataChart(payload: String) -> Element {
    let data: ChartData = match serde_json::from_str(&payload) {
        Ok(d) => d,
        Err(e) => return rsx! { div { class: "p-4 text-red-500", "Chart Error: {e}" } }
    };
    
    match data.chart_type.as_str() {
        "bar" => rsx! { BarChart { data: data } },
        "line" => rsx! { LineChart { data: data } },
        "pie" | "donut" => rsx! { PieChart { data: data } },
        _ => rsx! { div { class: "p-4 text-slate-400", "Unsupported chart type: {data.chart_type}" } }
    }
}

#[component]
fn BarChart(data: ChartData) -> Element {
    let max_val = data.data.iter().fold(0.0f64, |a, &b| a.max(b)).max(1.0);
    let bar_width = 100.0 / data.data.len() as f32;
    
    rsx! {
        div { class: "w-full p-6 space-y-4 bg-white rounded-2xl shadow-sm border border-slate-100",
            if let Some(title) = &data.title {
                h4 { class: "text-sm font-semibold text-slate-700", "{title}" }
            }
            div { class: "relative h-48 w-full flex items-end gap-2 px-2",
                // Grid lines
                for i in 0..=4 {
                    div { 
                        class: "absolute left-0 right-0 border-t border-slate-50",
                        style: "bottom: {i * 25}%"
                    }
                }
                
                for (i, &val) in data.data.iter().enumerate() {
                    {
                        let height = (val / max_val * 100.0) as f32;
                        let label = data.labels.get(i).cloned().unwrap_or_default();
                        rsx! {
                            div { class: "relative flex-1 group flex flex-col items-center",
                                // Bar
                                div { 
                                    class: "w-full bg-gradient-to-t from-blue-500 to-indigo-400 rounded-t-lg transition-all duration-300 hover:from-blue-600 hover:to-indigo-500 shadow-sm",
                                    style: "height: {height}%",
                                    title: "{val}"
                                }
                                // Tooltip
                                div { class: "absolute -top-8 bg-slate-800 text-white text-[10px] px-2 py-1 rounded opacity-0 group-hover:opacity-100 transition-opacity whitespace-nowrap z-10",
                                    "{val}"
                                }
                            }
                        }
                    }
                }
            }
            // Labels
            div { class: "flex justify-between px-2 gap-2",
                for label in data.labels.iter() {
                    span { class: "flex-1 text-[10px] text-slate-400 text-center truncate", "{label}" }
                }
            }
        }
    }
}

#[component]
fn LineChart(data: ChartData) -> Element {
    let max_val = data.data.iter().fold(0.0f64, |a, &b| a.max(b)).max(1.0);
    let width = 400.0f32;
    let height = 200.0f32;
    let padding = 20.0f32;
    
    let step_x = (width - padding * 2.0) / (data.data.len() - 1).max(1) as f32;
    
    let points: Vec<(f32, f32)> = data.data.iter().enumerate().map(|(i, &val)| {
        let x = padding + i as f32 * step_x;
        let y = height - padding - (val as f32 / max_val as f32 * (height - padding * 2.0));
        (x, y)
    }).collect();
    
    let mut path_data = String::new();
    if let Some(first) = points.first() {
        path_data.push_str(&format!("M {} {}", first.0, first.1));
        for p in points.iter().skip(1) {
            path_data.push_str(&format!(" L {} {}", p.0, p.1));
        }
    }

    rsx! {
        div { class: "w-full p-6 space-y-4 bg-white rounded-2xl shadow-sm border border-slate-100",
            if let Some(title) = &data.title {
                h4 { class: "text-sm font-semibold text-slate-700", "{title}" }
            }
            svg { 
                view_box: "0 0 {width} {height}",
                class: "w-full h-auto overflow-visible",
                
                // Background Grid
                for i in 0..=4 {
                    line { 
                        x1: "{padding}", y1: "{padding + i as f32 * (height - padding * 2.0) / 4.0}",
                        x2: "{width - padding}", y2: "{padding + i as f32 * (height - padding * 2.0) / 4.0}",
                        stroke: "#f1f5f9",
                        stroke_width: "1"
                    }
                }
                
                // Area fill
                path {
                    d: "{path_data} L {width - padding} {height - padding} L {padding} {height - padding} Z",
                    fill: "url(#line-gradient)",
                    fill_opacity: "0.1"
                }
                
                // Line
                path {
                    d: "{path_data}",
                    fill: "none",
                    stroke: "#6366f1",
                    stroke_width: "3",
                    stroke_linecap: "round",
                    stroke_linejoin: "round"
                }
                
                // Gradients
                defs {
                    linearGradient { id: "line-gradient", x1: "0%", y1: "0%", x2: "0%", y2: "100%",
                        stop { offset: "0%", stop_color: "#6366f1" }
                        stop { offset: "100%", stop_color: "#fff" }
                    }
                }
                
                // Points
                for p in points.iter() {
                    circle { cx: "{p.0}", cy: "{p.1}", r: "4", fill: "#6366f1", stroke: "#fff", stroke_width: "2" }
                }
            }
            // Labels
            div { class: "flex justify-between px-4",
                for label in data.labels.iter() {
                    span { class: "text-[10px] text-slate-400", "{label}" }
                }
            }
        }
    }
}

#[component]
fn PieChart(data: ChartData) -> Element {
    let total: f64 = data.data.iter().sum::<f64>().max(1.0);
    let mut current_angle = 0.0f64;
    let colors = vec!["#6366f1", "#3b82f6", "#06b6d4", "#10b981", "#f59e0b", "#ef4444"];
    
    rsx! {
        div { class: "w-full p-6 space-y-6 bg-white rounded-2xl shadow-sm border border-slate-100",
            if let Some(title) = &data.title {
                h4 { class: "text-sm font-semibold text-slate-700", "{title}" }
            }
            div { class: "flex flex-col md:flex-row items-center gap-8",
                svg { 
                    view_box: "0 0 100 100",
                    class: "size-48 overflow-visible",
                    
                    for (i, &val) in data.data.iter().enumerate() {
                        {
                            let slice_angle = (val / total) * 360.0;
                            let start_angle = current_angle;
                            current_angle += slice_angle;
                            
                            let start_rad = (start_angle - 90.0).to_radians();
                            let end_rad = (current_angle - 90.0).to_radians();
                            
                            let x1 = 50.0 + 40.0 * start_rad.cos();
                            let y1 = 50.0 + 40.0 * start_rad.sin();
                            let x2 = 50.0 + 40.0 * end_rad.cos();
                            let y2 = 50.0 + 40.0 * end_rad.sin();
                            
                            let large_arc = if slice_angle > 180.0 { 1 } else { 0 };
                            let color = colors[i % colors.len()];
                            
                            rsx! {
                                path {
                                    d: "M 50 50 L {x1} {y1} A 40 40 0 {large_arc} 1 {x2} {y2} Z",
                                    fill: "{color}",
                                    stroke: "#fff",
                                    stroke_width: "1"
                                }
                            }
                        }
                    }
                    // Inner hole (Donut)
                    circle { cx: "50", cy: "50", r: "20", fill: "#fff" }
                }
                
                // Legend
                div { class: "grid grid-cols-2 gap-x-6 gap-y-2",
                    for (i, label) in data.labels.iter().enumerate() {
                        div { class: "flex items-center gap-2",
                            div { class: "size-2 rounded-full", style: "background-color: {colors[i % colors.len()]}" }
                            span { class: "text-[10px] text-slate-500 truncate max-w-[100px]", "{label}" }
                        }
                    }
                }
            }
        }
    }
}
