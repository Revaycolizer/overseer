use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use crate::backend;
use crate::state::use_global_state;
use super::canvas::{CanvasNode, CanvasEdge, WorkflowCanvas};

#[component]
pub fn WorkflowAutomation() -> Element {
    let state = use_global_state();
    let db = state.read().db.clone();

    let db_for_wf = db.clone();
    let workflows = use_resource(move || {
        let d = db_for_wf.clone();
        async move { backend::get_workflows(&d).await.unwrap_or_default() }
    });

    let agent_system = state.read().agent_system.clone();
    drop(state);

    let mut selected_wf = use_signal(|| None::<i64>);
    let mut nodes = use_signal(|| vec![
        CanvasNode { id: 1, node_type: "manual_trigger".into(), label: "Trigger".into(), x: 40.0, y: 40.0 },
        CanvasNode { id: 2, node_type: "llm".into(), label: "LLM Call".into(), x: 280.0, y: 40.0 },
    ]);
    let mut edges = use_signal(|| vec![CanvasEdge { id: 1, from: 1, to: 2 }]);
    let mut selected_node = use_signal(|| None::<i64>);

    let wf_list = workflows.read().cloned().unwrap_or_default();

    rsx! {
        div { class: "flex-1 flex flex-col h-screen bg-slate-50",
            div { class: "border-b border-slate-200 bg-white px-4 py-3 flex items-center gap-4",
                select { class: "border border-slate-200 rounded-lg px-3 py-2 text-sm font-medium text-slate-700 outline-none focus:border-blue-400 transition-colors",
                    value: "{selected_wf().map(|id| id.to_string()).unwrap_or_default()}",
                    onchange: move |e| { if let Ok(id) = e.value().parse::<i64>() { selected_wf.set(Some(id)); } },
                    option { value: "", "Select workflow..." }
                    for wf in &wf_list {
                        option { value: "{wf.id}", selected: selected_wf() == Some(wf.id), "{wf.name}" }
                    }
                }
                button { class: "bg-blue-600 hover:bg-blue-700 text-white px-4 py-2 rounded-lg text-xs font-semibold transition-all active:scale-95",
                    onclick: {
                        let d = db.clone();
                        move |_| {
                            let d = d.clone();
                            spawn(async move {
                                if let Ok(id) = backend::add_workflow(&d, "New Workflow").await {
                                    selected_wf.set(Some(id));
                                }
                            });
                        }
                    },
                    MaterialIcon { name: "add", size: 16 }
                    "New"
                }
                button { class: "bg-emerald-600 hover:bg-emerald-700 text-white px-4 py-2 rounded-lg text-xs font-semibold transition-all active:scale-95",
                    onclick: move |_| {
                        let d = db.clone();
                        let a = agent_system.clone();
                        let wf = selected_wf();
                        spawn(async move {
                            let _ = backend::run_workflow(&d, &a, wf.unwrap_or(0)).await;
                        });
                    },
                    MaterialIcon { name: "play_arrow", size: 16 }
                    "Run"
                }
            }
            div { class: "flex-1 flex overflow-hidden",
                div { class: "w-56 border-r border-slate-200 bg-white p-3 space-y-2 overflow-y-auto shrink-0",
                    p { class: "text-[10px] font-bold text-slate-400 uppercase tracking-wider mb-3", "Nodes" }
                    {NODE_TYPES.iter().map(|(nt, label, icon)| {
                        let label = *label;
                        rsx! {
                            button { class: "w-full flex items-center gap-3 px-3 py-2 rounded-lg text-xs text-slate-600 hover:bg-blue-50 hover:text-blue-700 transition-colors border border-transparent hover:border-blue-100",
                                onclick: move |_| {
                                    let max_id = nodes.read().iter().map(|n| n.id).max().unwrap_or(0);
                                    nodes.write().push(CanvasNode { id: max_id + 1, node_type: nt.to_string(), label: label.to_string(), x: 40.0, y: 100.0 + max_id as f64 * 100.0 });
                                },
                                MaterialIcon { name: icon, size: 18 }
                                "{label}"
                            }
                        }
                    })}
                }
                WorkflowCanvas { nodes, edges, selected_node }
            }
        }
    }
}

const NODE_TYPES: &[(&str, &str, &str)] = &[
    ("manual_trigger", "Trigger", "power_settings_new"),
    ("chat_trigger", "Chat Trigger", "chat"),
    ("agent", "Agent", "smart_toy"),
    ("llm", "LLM Call", "psychology"),
    ("http_request", "HTTP Request", "http"),
    ("if", "Condition", "call_split"),
    ("set", "Set Value", "data_object"),
];
