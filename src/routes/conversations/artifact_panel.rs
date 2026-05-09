// src/routes/conversations/artifact_panel.rs
use super::artifact_card::ArtifactCard;
use crate::backend;
use dioxus::prelude::*;

#[component]
pub fn RecentArtifactsPanel(
    artifacts: Vec<backend::Artifact>,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    focused_artifact: Signal<Option<backend::Artifact>>,
    on_open: EventHandler<String>,
) -> Element {
    rsx! {
        div { class: "border-l border-slate-200 border-b border-slate-200 bg-white/95 backdrop-blur-sm",
            div { class: "px-5 py-4 space-y-3",
                div {
                    p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Recent Artifacts" }
                    h3 { class: "text-sm font-black text-slate-900 mt-1", "Outputs that agents can hand off, inspect, or reuse." }
                }

                if artifacts.is_empty() {
                    div { class: "rounded-2xl border border-dashed border-slate-200 bg-slate-50 px-4 py-3 text-xs text-slate-500 leading-relaxed",
                        "No artifacts recorded yet. Screenshot files, links, and machine-state outputs will show up here after a run produces them."
                    }
                } else {
                    div { class: "space-y-2 max-h-72 overflow-y-auto custom-scrollbar pr-1",
                        for artifact in artifacts {
                            ArtifactCard {
                                key: "{artifact.id}",
                                artifact: artifact,
                                selected_artifacts: selected_artifacts,
                                focused_artifact: focused_artifact,
                                on_open: on_open.clone(),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn ArtifactKindBadge(kind: String) -> Element {
    let (label, class) = match kind.as_str() {
        "image" => ("Image", "bg-blue-50 text-blue-700 border-blue-200"),
        "file" => ("File", "bg-emerald-50 text-emerald-700 border-emerald-200"),
        "link" => ("Link", "bg-violet-50 text-violet-700 border-violet-200"),
        "record" => ("Record", "bg-amber-50 text-amber-700 border-amber-200"),
        _ => ("Note", "bg-slate-100 text-slate-600 border-slate-200"),
    };

    rsx! {
        span { class: format!("inline-flex items-center rounded-full border px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] {}", class),
            "{label}"
        }
    }
}
