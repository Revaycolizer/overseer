use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;
use base64::{engine::general_purpose, Engine as _};
use crate::backend;

#[component]
pub fn ArtifactWorkspaceSurface(
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    starter_prompt: Signal<Option<String>>,
) -> Element {
    let artifacts = selected_artifacts();
    let actions = build_artifact_workspace_actions(&artifacts);
    rsx! {
        div { class: "mb-3 overflow-hidden rounded-2xl border border-emerald-200 bg-gradient-to-br from-emerald-50 via-white to-blue-50 shadow-sm",
            div { class: "border-b border-emerald-100 px-4 py-3",
                div { class: "flex flex-wrap items-start justify-between gap-3",
                    div { class: "space-y-1",
                        p { class: "text-[10px] font-black uppercase tracking-[0.2em] text-emerald-600", "Artifact Workspace" }
                        h3 { class: "text-sm font-black text-slate-900", "Selected artifacts will guide the next task." }
                        p { class: "text-xs text-slate-600 leading-relaxed", "Use these as explicit working context instead of relying on chat memory alone." }
                    }
                    button { class: "inline-flex items-center gap-2 rounded-xl border border-emerald-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] text-emerald-700 hover:border-emerald-300 hover:bg-emerald-50 transition-all",
                        onclick: move |_| selected_artifacts.set(Vec::new()),
                        MaterialIcon { name: "layers_clear", size: 14 }
                        "Clear Selection"
                    }
                }
            }
            div { class: "space-y-3 px-4 py-4",
                div { class: "grid gap-3",
                    for artifact in artifacts.iter() {
                        ArtifactWorkspaceCard { key: "{artifact.id}", artifact: artifact.clone(), selected_artifacts: selected_artifacts }
                    }
                }
                if !actions.is_empty() {
                    div { class: "space-y-2 pt-1",
                        p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "Suggested Next Moves" }
                        div { class: "flex flex-wrap gap-2",
                            for (label, icon, prompt) in actions {
                                button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.14em] text-slate-700 hover:border-blue-200 hover:bg-blue-50 hover:text-blue-700 transition-all",
                                    onclick: { let p = prompt.clone(); move |_| starter_prompt.set(Some(p.clone())) },
                                    MaterialIcon { name: icon, size: 14 }
                                    "{label}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ArtifactWorkspaceCard(
    artifact: backend::Artifact,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
) -> Element {
    let kind = artifact.kind.clone();
    let path = artifact.file_path.clone();
    let ct = artifact.content_type.clone().unwrap_or_else(|| "image/png".to_string());
    let preview = use_resource(move || {
        let k = kind.clone(); let p = path.clone(); let c = ct.clone();
        async move {
            if k != "image" { return None; }
            let p = p?;
            let bytes = std::fs::read(&p).ok().filter(|b| !b.is_empty())?;
            Some(format!("data:{};base64,{}", c, general_purpose::STANDARD.encode(bytes)))
        }
    });
    let domain = artifact.domain.clone().filter(|v| !v.trim().is_empty()).unwrap_or_else(|| "general".to_string()).replace('_', " ");
    rsx! {
        div { class: "rounded-2xl border border-white/80 bg-white/90 px-4 py-4 shadow-sm",
            div { class: "flex items-start justify-between gap-3",
                div { class: "min-w-0 space-y-1",
                    div { class: "flex flex-wrap items-center gap-2",
                        span { class: "inline-flex items-center rounded-full border border-emerald-200 bg-emerald-50 px-2.5 py-1 text-[10px] font-black uppercase tracking-[0.18em] text-emerald-700", "{artifact.kind.to_uppercase()}" }
                        span { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "{domain}" }
                    }
                    p { class: "text-sm font-black text-slate-900 break-words", "{artifact.label}" }
                    if let Some(ref s) = artifact.step_text.clone().filter(|v| !v.trim().is_empty()) {
                        p { class: "text-xs text-slate-500 leading-relaxed", "{s}" }
                    }
                }
                button { class: "rounded-xl border border-slate-200 bg-slate-50 p-2 text-slate-400 hover:border-red-200 hover:bg-red-50 hover:text-red-600 transition-all",
                    onclick: { let id = artifact.id; move |_| selected_artifacts.write().retain(|item| item.id != id) },
                    MaterialIcon { name: "close", size: 16 }
                }
            }
            if artifact.kind == "image" {
                match preview().flatten() {
                    Some(src) => rsx! { img { src: "{src}", alt: "{artifact.label}", class: "mt-3 max-h-48 rounded-xl border border-slate-200 bg-slate-50 object-contain" } },
                    None => rsx! { div { class: "mt-3 rounded-xl border border-dashed border-slate-200 bg-slate-50 px-4 py-5 text-xs text-slate-400", "Image preview unavailable." } },
                }
            } else if let Some(p) = artifact.file_path.clone().filter(|v| !v.trim().is_empty()) {
                p { class: "mt-3 text-[11px] font-mono text-slate-500 break-all", "{p}" }
            } else {
                p { class: "mt-3 text-xs text-slate-400", "This artifact is selected as context for the next task." }
            }
        }
    }
}

fn build_artifact_workspace_actions(artifacts: &[backend::Artifact]) -> Vec<(String, String, String)> {
    if artifacts.is_empty() { return vec![]; }
    if artifacts.len() == 1 {
        let a = &artifacts[0];
        let mut actions = vec![("Summarize It".into(), "summarize".into(), "Summarize the selected artifact and tell me what matters most.".into())];
        match a.kind.as_str() {
            "image" => actions.push(("Analyze Screenshot".into(), "image_search".into(), "Analyze the selected screenshot and describe what is visible, important, or unusual.".into())),
            "file" => actions.push(("Read File".into(), "description".into(), "Read the selected file and summarize its contents.".into())),
            "record" => actions.push(("Extract Facts".into(), "fact_check".into(), "Extract the key facts from the selected artifact in a concise list.".into())),
            _ => {}
        }
        actions.push(("Turn Into Task".into(), "conversion_path".into(), "Use the selected artifact as context and help me decide the best next action.".into()));
        return actions;
    }
    vec![
        ("Compare Selected".into(), "compare".into(), "Compare the selected artifacts and explain the important differences.".into()),
        ("Synthesize Report".into(), "auto_awesome".into(), "Synthesize the selected artifacts into one clear report with the key takeaways.".into()),
        ("Plan Next Step".into(), "account_tree".into(), "Use the selected artifacts as context and propose the best next step.".into()),
    ]
}
