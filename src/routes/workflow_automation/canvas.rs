use dioxus::prelude::*;

#[derive(Clone, PartialEq)]
pub struct CanvasNode {
    pub id: i64,
    pub node_type: String,
    pub label: String,
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, PartialEq)]
pub struct CanvasEdge {
    pub id: i64,
    pub from: i64,
    pub to: i64,
}

fn node_color(node_type: &str) -> &'static str {
    match node_type {
        t if t.contains("trigger") || t.contains("chat") => "#10b981",
        t if t.contains("browser") => "#8b5cf6",
        t if t.contains("agent") || t == "llm" => "#3b82f6",
        t if t.contains("http") => "#f59e0b",
        t if t.contains("if") => "#ef4444",
        _ => "#64748b",
    }
}

fn node_icon(node_type: &str) -> &'static str {
    match node_type {
        "manual_trigger" => "⚡" , "chat_trigger" => "💬", "agent" => "🤖", "llm" => "🧠",
        "http_request" => "🌐", "if" => "❓", "set" => "📦",
        "browser_session" => "🌍", "browser_navigate" => "🔗", "browser_click" => "👆",
        "browser_screenshot" => "📷", "browser_extract" => "📝", "rss_feed" => "📡", "csv_parse" => "📊",
        _ => "📌",
    }
}

#[component]
pub fn WorkflowCanvas(
    nodes: Signal<Vec<CanvasNode>>,
    edges: Signal<Vec<CanvasEdge>>,
    selected_node: Signal<Option<i64>>,
) -> Element {
    let mut pan_x = use_signal(|| 0.0f64);
    let mut pan_y = use_signal(|| 0.0f64);
    let mut zoom = use_signal(|| 1.0f64);
    let mut panning = use_signal(|| false);
    let mut drag_nid = use_signal(|| None::<i64>);
    let mut connect_from = use_signal(|| None::<i64>);
    let mut mouse_pos = use_signal(|| (0.0f64, 0.0f64));
    let mut last_mouse = use_signal(|| (0.0f64, 0.0f64));

    let sel = selected_node();
    let nodes_v: Vec<(i64, String, String, f64, f64, bool, &'static str, &'static str)> = {
        nodes.read().iter().map(|n| (n.id, n.node_type.clone(), n.label.clone(), n.x, n.y, sel == Some(n.id), node_color(&n.node_type), node_icon(&n.node_type))).collect()
    };
    let edges_v: Vec<(i64, String)> = {
        let nm: std::collections::HashMap<i64, (f64, f64)> = nodes.read().iter().map(|n| (n.id, (n.x, n.y))).collect();
        edges.read().iter().map(|e| {
            let d = nm.get(&e.from).zip(nm.get(&e.to)).map(|(f, t)| {
                format!("M{:.0} {:.0} C{:.0} {:.0} {:.0} {:.0} {:.0} {:.0}", f.0 + 90.0, f.1 + 32.0, f.0 + 90.0, f.1 + 80.0, t.0 + 90.0, t.1 - 10.0, t.0 + 90.0, t.1)
            }).unwrap_or_default();
            (e.id, d)
        }).collect()
    };
    let connect_line = connect_from().and_then(|from_id| {
        nodes_v.iter().find(|(id, _, _, _, _, _, _, _)| *id == from_id).map(|(_, _, _, x, y, _, _, _)| {
            format!("M{:.0} {:.0} L{:.0} {:.0}", x + 90.0, y + 80.0, mouse_pos().0, mouse_pos().1)
        })
    });

    rsx! {
        div {
            class: "flex-1 relative overflow-hidden bg-[#f1f5f9]",
            style: "background-image: radial-gradient(#e2e8f0 1px, transparent 1px); background-size: 24px 24px;",
            onmousedown: move |e| {
                let c = e.data().client_coordinates();
                last_mouse.set((c.x as f64, c.y as f64));
                connect_from.set(None);
            },
            onmousemove: move |e| {
                let c = e.data().client_coordinates();
                mouse_pos.set((c.x as f64, c.y as f64));
                let (lx, ly) = last_mouse();
                let dx = c.x as f64 - lx;
                let dy = c.y as f64 - ly;
                if panning() { pan_x += dx; pan_y += dy; }
                if let Some(nid) = drag_nid() {
                    let mut n = nodes.write();
                    if let Some(node) = n.iter_mut().find(|n| n.id == nid) {
                        node.x += dx / zoom();
                        node.y += dy / zoom();
                    }
                }
                last_mouse.set((c.x as f64, c.y as f64));
            },
            onmouseup: move |_| {
                panning.set(false); drag_nid.set(None);
                if let Some(from_id) = connect_from() {
                    let c = mouse_pos();
                    let target = nodes_v.iter().find(|(_, _, _, x, y, _, _, _)| {
                        (c.0 - (x + 90.0)).abs() < 90.0 && (c.1 - (y + 40.0)).abs() < 50.0
                    }).map(|(id, _, _, _, _, _, _, _)| *id);
                    if let Some(to_id) = target {
                        if from_id != to_id {
                            let mut e = edges.write();
                            let max_id = e.iter().map(|e| e.id).max().unwrap_or(0);
                            e.push(CanvasEdge { id: max_id + 1, from: from_id, to: to_id });
                        }
                    }
                    connect_from.set(None);
                }
            },
            // Zoom buttons instead of wheel — wheel delta API varies by platform
            div { class: "absolute top-12 left-3 z-10 flex flex-col gap-1",
                button { class: "bg-white border border-slate-200 rounded-lg size-7 flex items-center justify-center text-xs font-bold text-slate-600 hover:bg-slate-50 transition-colors shadow-sm",
                    onclick: move |_| zoom.set((zoom() * 1.2).min(3.0)), "+" }
                button { class: "bg-white border border-slate-200 rounded-lg size-7 flex items-center justify-center text-xs font-bold text-slate-600 hover:bg-slate-50 transition-colors shadow-sm",
                    onclick: move |_| zoom.set((zoom() / 1.2).max(0.25)), "−" }
            }
            div { class: "absolute top-3 left-3 z-10 flex items-center gap-3 bg-white/90 backdrop-blur-sm rounded-xl border border-slate-200 px-3 py-2 shadow-sm",
                span { class: "text-xs font-medium text-slate-500", "Zoom: {((zoom() * 100.0) as i32)}%" }
                div { class: "w-px h-4 bg-slate-200" }
                span { class: "text-xs text-slate-400", "Drag canvas · Scroll to zoom"}
            }
            svg { class: "w-full h-full",
                g { transform: format!("translate({:.0},{:.0}) scale({:.2})", pan_x(), pan_y(), zoom()),
                    defs {
                        marker { id: "arrow", marker_width: 8, marker_height: 6, ref_x: 8, ref_y: 3, orient: "auto",
                            polygon { points: "0 0, 8 3, 0 6", fill: "#94a3b8" }
                        }
                    }
                    for (_, d) in edges_v.clone() {
                        path { d: "{d}", fill: "none", stroke: "#94a3b8", stroke_width: 1.5, opacity: 0.6, marker_end: "url(#arrow)",
                            style: "transition: stroke 0.15s, opacity 0.15s; cursor: pointer;" }
                    }
                    {connect_line.map(|d| rsx! {
                        path { d: "{d}", fill: "none", stroke: "#3b82f6", stroke_width: 2, stroke_dasharray: "6,3" }
                    })}
                    for (nid, nt, label, x, y, is_sel, color, icon) in nodes_v.clone() {
                        g {
                            transform: format!("translate({x:.0},{y:.0})"),
                            onpointerdown: move |e| { e.stop_propagation(); drag_nid.set(Some(nid)); selected_node.set(Some(nid)); },
                            rect { x: -1, y: -1, width: 182, height: 82, rx: 11, ry: 11, fill: "rgba(0,0,0,0.04)" }
                            rect { width: 180, height: 80, rx: 10, ry: 10, fill: "white", stroke: if is_sel { "#3b82f6" } else { "#e2e8f0" }, stroke_width: if is_sel { 2 } else { 1 },
                                style: "transition: stroke 0.15s, stroke-width 0.15s; cursor: grab;" }
                            rect { width: 180, height: 32, rx: 10, ry: 10, fill: color }
                            rect { width: 180, height: 10, y: 22, fill: color }
                            text { x: 10, y: 22, fill: "white", font_size: 14, "{icon}" }
                            text { x: 30, y: 22, fill: "white", font_size: 12, font_weight: "bold", "{label}" }
                            text { x: 12, y: 56, fill: "#64748b", font_size: 10, "{nt}" }
                            circle { cx: 90, cy: 0, r: 5, fill: color, stroke: "white", stroke_width: 2, cursor: "crosshair",
                                onclick: move |e| { e.stop_propagation(); connect_from.set(Some(nid)); } }
                            circle { cx: 90, cy: 80, r: 5, fill: color, stroke: "white", stroke_width: 2, cursor: "crosshair",
                                onclick: move |e| { e.stop_propagation(); connect_from.set(Some(nid)); } }
                        }
                    }
                }
            }
        }
    }
}
