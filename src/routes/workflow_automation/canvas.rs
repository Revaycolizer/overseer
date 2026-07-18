use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

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

#[component]
pub fn WorkflowCanvas(
    nodes: Signal<Vec<CanvasNode>>,
    edges: Signal<Vec<CanvasEdge>>,
    selected_node: Signal<Option<i64>>,
) -> Element {
    let mut offset_x = use_signal(|| 0.0f64);
    let mut offset_y = use_signal(|| 0.0f64);
    let mut zoom = use_signal(|| 1.0f64);

    let node_data: Vec<(i64, String, String, f64, f64)> = nodes.read().iter()
        .map(|n| (n.id, n.node_type.clone(), n.label.clone(), n.x, n.y)).collect();
    let edge_data: Vec<(i64, i64)> = edges.read().iter().map(|e| (e.from, e.to)).collect();

    let edge_paths: Vec<String> = edge_data.iter().filter_map(|(from_id, to_id)| {
        let from = node_data.iter().find(|(id, _, _, _, _)| *id == *from_id);
        let to = node_data.iter().find(|(id, _, _, _, _)| *id == *to_id);
        from.zip(to).map(|(f, t)| {
            let x1 = f.3 + 90.0; let y1 = f.4 + 32.0;
            let x2 = t.3 + 90.0; let y2 = t.4;
            let cy = (y1 + y2) / 2.0;
            format!("M{x1:.0} {y1:.0} C{x1:.0} {cy:.0} {x2:.0} {cy:.0} {x2:.0} {y2:.0}")
        })
    }).collect();

    let sel = selected_node();
    let node_list: Vec<(i64, String, String, f64, f64, bool)> = node_data.into_iter().map(|(id, t, l, x, y)| (id, t, l, x, y, sel == Some(id))).collect();

    rsx! {
        div { class: "flex-1 relative overflow-hidden bg-slate-50/50",
            div { class: "absolute top-3 left-3 flex gap-2 z-10",
                span { class: "text-xs text-slate-400 self-center", "Zoom: {((zoom() * 100.0) as i32)}%" }
            }
            svg { class: "w-full h-full",
                g {
                    transform: format!("translate({:.0}, {:.0}) scale({:.2})", offset_x(), offset_y(), zoom()),
                    for d in edge_paths {
                        path { d: "{d}", fill: "none", stroke: "#94a3b8", stroke_width: 2 }
                    }
                    for (id, node_type, label, x, y, is_sel) in node_list {
                        g {
                            transform: format!("translate({x:.0}, {y:.0})"),
                            rect { width: 180, height: 80, rx: 10, ry: 10, fill: "white", stroke: if is_sel { "#3b82f6" } else { "#e2e8f0" }, stroke_width: 2 }
                            rect { width: 180, height: 32, rx: 10, ry: 10, fill: if is_sel { "#3b82f6" } else { "#1e293b" } }
                            text { x: 12, y: 21, fill: "white", font_size: 12, font_weight: "bold", "{label}" }
                            text { x: 12, y: 56, fill: "#64748b", font_size: 10, "{node_type}" }
                        }
                    }
                }
            }
        }
    }
}
