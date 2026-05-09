// src/routes/conversations/artifact_card.rs
use super::artifact_panel::ArtifactKindBadge;
use super::helpers::copy_to_clipboard;
use super::route_summary::prettify_token;
use base64::{engine::general_purpose, Engine as _};
use crate::backend;
use dioxus::prelude::*;
use dioxus_material_icons::MaterialIcon;

#[component]
pub fn ArtifactCard(
    artifact: backend::Artifact,
    selected_artifacts: Signal<Vec<backend::Artifact>>,
    focused_artifact: Signal<Option<backend::Artifact>>,
    on_open: EventHandler<String>,
) -> Element {
    let artifact_kind = artifact.kind.clone();
    let artifact_content_type = artifact.content_type.clone();
    let file_path = artifact.file_path.clone();
    let is_selected = selected_artifacts().iter().any(|s| s.id == artifact.id);
    let is_focused = focused_artifact().as_ref().map(|f| f.id == artifact.id).unwrap_or(false);
    let image_preview = use_resource(move || {
        let kind = artifact_kind.clone();
        let file_path = file_path.clone();
        let content_type = artifact_content_type.clone().unwrap_or_else(|| "image/png".to_string());
        async move {
            if kind != "image" { return None; }
            let Some(path) = file_path else { return None; };
            match std::fs::read(&path) {
                Ok(bytes) if !bytes.is_empty() => Some(format!("data:{};base64,{}", content_type, general_purpose::STANDARD.encode(bytes))),
                _ => None,
            }
        }
    });

    rsx! {
        div { class: format!("rounded-2xl border px-4 py-3 space-y-3 transition-all {}", if is_focused { "border-blue-200 bg-blue-50/60 shadow-sm" } else { "border-slate-200 bg-slate-50/80" }),
            ArtifactCardHeader { artifact: artifact.clone(), is_selected: is_selected, is_focused: is_focused, selected_artifacts: selected_artifacts, focused_artifact: focused_artifact }
            ArtifactCardBody { artifact: artifact.clone(), image_preview: image_preview, on_open: on_open }
        }
    }
}

#[component]
fn ArtifactCardHeader(artifact: backend::Artifact, is_selected: bool, is_focused: bool, selected_artifacts: Signal<Vec<backend::Artifact>>, focused_artifact: Signal<Option<backend::Artifact>>) -> Element {
    rsx! {
        div { class: "flex items-start justify-between gap-3",
            div { class: "min-w-0",
                p { class: "text-xs font-black text-slate-900 truncate", "{artifact.label}" }
                p { class: "text-[11px] text-slate-400 mt-1", {artifact.created_at.format("%Y-%m-%d %H:%M").to_string()} }
            }
            ArtifactKindBadge { kind: artifact.kind.clone() }
        }
        button {
            class: format!("inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] transition-all {}", if is_selected { "border-blue-200 bg-blue-50 text-blue-700" } else { "border-slate-200 bg-white text-slate-600 hover:border-blue-200 hover:text-blue-700" }),
            onclick: { let artifact = artifact.clone(); move |_| { selected_artifacts.with_mut(|s| { if let Some(i) = s.iter().position(|x| x.id == artifact.id) { s.remove(i); } else { s.push(artifact.clone()); } }); } },
            MaterialIcon { name: if is_selected { "task_alt" } else { "add_circle" }, size: 14 }
            if is_selected { "Selected for Next Prompt" } else { "Use in Next Prompt" }
        }
        button {
            class: format!("inline-flex items-center gap-2 rounded-xl border px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] transition-all {}", if is_focused { "border-blue-200 bg-white text-blue-700" } else { "border-slate-200 bg-white text-slate-600 hover:border-blue-200 hover:text-blue-700" }),
            onclick: { let artifact = artifact.clone(); move |_| focused_artifact.set(Some(artifact.clone())) },
            MaterialIcon { name: "visibility", size: 14 }
            if is_focused { "Session Open" } else { "Open Session" }
        }
        if let Some(domain) = artifact.domain.clone().filter(|v| !v.trim().is_empty()) {
            p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-slate-400", "{prettify_token(&domain)}" }
        }
        if let Some(step_text) = artifact.step_text.clone().filter(|v| !v.trim().is_empty()) {
            p { class: "text-xs text-slate-600 line-clamp-2", "{step_text}" }
        }
    }
}

#[component]
fn ArtifactCardBody(artifact: backend::Artifact, image_preview: Resource<Option<String>>, on_open: EventHandler<String>) -> Element {
    rsx! {
        if artifact.kind == "image" {
            match image_preview().flatten() {
                Some(src) => rsx! { img { src: "{src}", alt: "{artifact.label}", class: "rounded-xl border border-slate-200 bg-white shadow-sm max-w-full h-auto block" } },
                None => rsx! { div { class: "rounded-xl border border-dashed border-slate-200 bg-white px-4 py-5 text-xs text-slate-400", "Image preview unavailable." } },
            }
        }
        if let Some(path) = artifact.file_path.clone().filter(|v| !v.trim().is_empty()) {
            p { class: "text-[11px] font-mono text-slate-500 break-all", "{path}" }
            div { class: "flex flex-wrap gap-2",
                button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] text-slate-600 hover:border-blue-200 hover:text-blue-700 transition-all",
                    onclick: { let path = path.clone(); move |_| on_open.call(path.clone()) }, MaterialIcon { name: "open_in_new", size: 14 } "Open" }
                button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] text-slate-600 hover:border-blue-200 hover:text-blue-700 transition-all",
                    onclick: { let path = path.clone(); move |_| copy_to_clipboard(path.clone()) }, MaterialIcon { name: "content_copy", size: 14 } "Copy Path" }
            }
        } else if let Some(content) = artifact.content.clone().filter(|v| !v.trim().is_empty()) {
            p { class: "text-xs text-slate-600 whitespace-pre-wrap break-words line-clamp-3", "{content}" }
            button { class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white px-3 py-2 text-[10px] font-black uppercase tracking-[0.18em] text-slate-600 hover:border-blue-200 hover:text-blue-700 transition-all",
                onclick: { let content = content.clone(); move |_| copy_to_clipboard(content.clone()) }, MaterialIcon { name: "content_copy", size: 14 } "Copy" }
        }
    }
}
