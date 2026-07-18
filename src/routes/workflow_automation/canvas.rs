use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

const NODE_WIDTH: f64 = 180.0;
const NODE_HEADER_H: f64 = 32.0;
const PORT_R: f64 = 6.0;
const GRID: f64 = 20.0;

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
    let mut offset = use_signal(|| (0.0f64, 0.0f64));
    let mut scale = use_signal(|| 1.0f64);
    let mut panning = use_signal(|| false);
    let mut drag_node = use_signal(|| None::<i64>);

    rsx! {
        div { class: "flex-1 relative overflow-hidden bg-slate-50/50",
            svg {
                class: "w-full h-full cursor-grab active:cursor-grabbing",
                onmousedown: move |e| {
                    if e.target().map(|t| t.class_name()).flatten().map(|c| c.contains("node")).unwrap_or(false) {
                        // handled by node mousedown
                    } else {
                        panning.set(true);
                    }
                },
                onmousemove: move |e| {
                    if panning() {
                        let (ox, oy) = offset();
                        offset.set((ox + e.movement_x() as f64, oy + e.movement_y() as f64));
                    }
                    if let Some(drag_id) = drag_node() {
                        let mut n = nodes.write();
                        if let Some(node) = n.iter_mut().find(|n| n.id == drag_id) {
                            node.x += e.movement_x() as f64 / scale();
                            node.y += e.movement_y() as f64 / scale();
                        }
                    }
                },
                onmouseup: move |_| { panning.set(false); drag_node.set(None); },
                onwheel: move |e| {
                    let s = scale();
                    let new_s = (s - e.delta_y() as f64 * 0.001).clamp(0.25, 3.0);
                    scale.set(new_s);
                },
                g {
                    transform: format!("translate({:.0}, {:.0}) scale({:.2})", offset().0, offset().1, scale()),
                    // Grid
                    rect { width: "100%", height: "100%", fill: "none" }
                    // Edges
                    for edge in edges.read().iter() {
                        let from = nodes.read().iter().find(|n| n.id == edge.from);
                        let to = nodes.read().iter().find(|n| n.id == edge.to);
                        if let (Some(f), Some(t)) = (from, to) {
                            let x1 = f.x + NODE_WIDTH / 2.0;
                            let y1 = f.y + NODE_HEADER_H;
                            let x2 = t.x + NODE_WIDTH / 2.0;
                            let y2 = t.y;
                            let cy = (y1 + y2) / 2.0;
                            path {
                                d: format!("M {x1:.0} {y1:.0} C {x1:.0} {cy:.0}, {x2:.0} {cy:.0}, {x2:.0} {y2:.0}"),
                                fill: "none", stroke: "#94a3b8", stroke_width: 2,
                                marker_end: "url(#arrowhead)"
                            }
                        }
                    }
                    // Nodes
                    for node in nodes.read().iter() {
                        g {
                            class: "node cursor-pointer",
                            transform: format!("translate({:.0}, {:.0})", node.x, node.y),
                            onmousedown: move |e| {
                                e.stop_propagation();
                                drag_node.set(Some(node.id));
                            },
                            onclick: move |_| selected_node.set(Some(node.id)),
                            rect { width: NODE_WIDTH, height: NODE_HEAD_H, rx: 8, ry: 8,
                                fill: if selected_node() == Some(node.id) { "#3b82f6" } else { "#1e293b" }
                            }
                            text { x: 12, y: 21, fill: "white", font_size: 12, font_weight: "bold",
                                "{node.label}"
                            }
                            rect { y: NODE_HEAD_H, width: NODE_WIDTH, height: NODE_HEAD_H * 1.5, rx: 8, ry: 8,
                                fill: "white", stroke: "#e2e8f0", stroke_width: 1
                            }
                            text { x: 12, y: NODE_HEAD_H + 24, fill: "#64748b", font_size: 10,
                                "{node.node_type}"
                            }
                            // Input port
                            circle { cx: NODE_WIDTH / 2.0, cy: 0, r: PORT_R, fill: "#3b82f6", stroke: "white", stroke_width: 2 }
                            // Output port
                            circle { cx: NODE_WIDTH / 2.0, cy: NODE_HEAD_H + NODE_HEAD_H * 1.5, r: PORT_R, fill: "#3b82f6", stroke: "white", stroke_width: 2 }
                        }
                    }
                }
                defs {
                    marker { id: "arrowhead", markerWidth: 10, markerHeight: 7, refX: 10, refY: 3.5, orient: "auto",
                        polygon { points: "0 0, 10 3.5, 0 7", fill: "#94a3b8" }
                    }
                }
            }
        }
    }
}
