use crate::backend;
use crate::state::use_global_state;
use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};
use super::artifact_workspace::ArtifactWorkspaceSurface;
use super::chat_composer::ChatComposer;

#[derive(Props, Clone, PartialEq)]
pub struct ChatInputProps {
    pub selected_conversation_id: Signal<Option<i64>>,
    pub messages_resource: Resource<Vec<backend::Message>>,
    pub visible_messages: Signal<Vec<backend::Message>>,
    pub conversations_resource: Resource<Vec<backend::Conversation>>,
    pub selected_artifacts: Signal<Vec<backend::Artifact>>,
    pub selected_work_item: Signal<Option<backend::WorkItem>>,
    pub is_first_msg: bool,
    pub is_thinking: Signal<bool>,
    pub starter_prompt: Signal<Option<String>>,
    /// Render the composer large and centered (empty / landing state).
    #[props(default)]
    pub large: bool,
}

#[component]
pub fn ChatInput(props: ChatInputProps) -> Element {
    let global_state = use_global_state();
    let global_read = global_state.read();
    let db = global_read.db.clone();
    drop(global_read);

    let selected_artifacts = props.selected_artifacts;
    let mut selected_work_item = props.selected_work_item;
    let selected_conversation_id = props.selected_conversation_id;
    let starter_prompt = props.starter_prompt;
    let selected_artifact_count = selected_artifacts().len();
    let has_active_work_item = selected_work_item().is_some();
    let db_for_computer_state = db.clone();

    let computer_state_resource = use_resource(move || {
        let db = db_for_computer_state.clone();
        async move { backend::get_computer_supervision_state(&db).await }
    });

    let computer_state = computer_state_resource.read().as_ref().and_then(|r| r.as_ref().ok().cloned());
    let supervision_mode = computer_state
        .as_ref()
        .map(|state| state.supervision_mode.clone())
        .unwrap_or_else(|| "approval_required".to_string());
    let session_state = computer_state
        .as_ref()
        .map(|state| state.session_state.clone())
        .unwrap_or_else(|| "idle".to_string());
    let approval_state = computer_state
        .as_ref()
        .map(|state| state.approval_state.clone())
        .unwrap_or_else(|| "idle".to_string());
    let pending_action = computer_state
        .as_ref()
        .map(|state| state.pending_action.clone())
        .unwrap_or_default();
    let session_label = match session_state.as_str() {
        "approval_session" => "Approval session",
        "observing" => "Observe mode",
        "trusted_session" => "Trusted session",
        "paused" => "Paused",
        "emergency_stopped" => "Emergency stop",
        _ => "No active session",
    };

    rsx! {
        div { class: "p-3 sm:p-6 bg-gradient-to-t from-[var(--workspace-bg)] via-[var(--workspace-bg)]/95 to-transparent",
            div { class: "w-full max-w-3xl mx-auto",
                if selected_artifact_count > 0 {
                    ArtifactWorkspaceSurface {
                        selected_artifacts: selected_artifacts,
                        starter_prompt: starter_prompt,
                    }
                }
                if let Some(work_item) = selected_work_item() {
                    div { class: "mb-3 flex items-start justify-between gap-3 rounded-2xl border border-violet-200 bg-violet-50/80 px-4 py-3 shadow-sm",
                        div { class: "min-w-0 space-y-1",
                            p { class: "text-[10px] font-black uppercase tracking-[0.18em] text-violet-600", "Active Work Item" }
                            p { class: "text-sm font-black text-slate-900 break-words", "{work_item.title}" }
                            p { class: "text-xs text-slate-600 line-clamp-2", "{work_item.prompt}" }
                        }
                        button {
                            class: "rounded-xl border border-violet-200 bg-white p-2 text-violet-400 hover:bg-violet-100 hover:text-violet-700 transition-all",
                            onclick: move |_| selected_work_item.set(None),
                            MaterialIcon { name: "close", size: 16 }
                        }
                    }
                }

                ChatComposer {
                    selected_conversation_id: selected_conversation_id,
                    messages_resource: props.messages_resource,
                    visible_messages: props.visible_messages,
                    conversations_resource: props.conversations_resource,
                    selected_artifacts: selected_artifacts,
                    selected_work_item: selected_work_item,
                    is_thinking: props.is_thinking,
                    starter_prompt: props.starter_prompt,
                    is_first_msg: props.is_first_msg,
                }

                // Mode / session controls moved to Settings. Only the per-action
                // approval prompt appears here, and only when an action is pending.
                if approval_state == "pending" && !pending_action.trim().is_empty() {
                    div { class: "mt-3 flex flex-wrap items-center gap-2 px-1",
                        div { class: "flex items-center gap-2 min-w-full rounded-2xl border border-amber-200 bg-amber-50/80 px-3 py-3 text-[11px] text-amber-900 shadow-sm",
                            MaterialIcon { name: "fact_check", size: 16, color: MaterialIconColor::Custom("#b45309".into()) }
                            p { class: "flex-1 leading-relaxed break-words", "{pending_action}" }
                            button {
                                class: "inline-flex items-center gap-2 rounded-xl bg-emerald-600 px-3 py-2 font-bold text-white hover:bg-emerald-700 transition-all",
                                onclick: {
                                    let db = db.clone();
                                    let computer_state_resource = computer_state_resource.clone();
                                    move |_| {
                                        let db = db.clone();
                                        let mut computer_state_resource = computer_state_resource.clone();
                                        spawn(async move {
                                            let _ = backend::approve_pending_computer_action(&db).await;
                                            computer_state_resource.restart();
                                        });
                                    }
                                },
                                MaterialIcon { name: "task_alt", size: 14, color: MaterialIconColor::Light }
                                "Approve"
                            }
                            button {
                                class: "inline-flex items-center gap-2 rounded-xl border border-red-200 bg-white px-3 py-2 font-bold text-red-600 hover:bg-red-50 transition-all",
                                onclick: {
                                    let db = db.clone();
                                    let computer_state_resource = computer_state_resource.clone();
                                    move |_| {
                                        let db = db.clone();
                                        let mut computer_state_resource = computer_state_resource.clone();
                                        spawn(async move {
                                            let _ = backend::reject_pending_computer_action(&db).await;
                                            computer_state_resource.restart();
                                        });
                                    }
                                },
                                MaterialIcon { name: "cancel", size: 14, color: MaterialIconColor::Custom("#dc2626".into()) }
                                "Reject"
                            }
                        }
                    }
                }
                div { class: "mt-2 flex items-center justify-between gap-3 px-1 text-[11px] text-slate-400",
                    div { class: "flex items-center gap-2 min-w-0",
                        if has_active_work_item {
                            MaterialIcon { name: "radar", size: 14 }
                            span { "Includes the active work item as context." }
                        } else if selected_artifact_count > 0 {
                            MaterialIcon { name: "radar", size: 14 }
                            span { "Includes {selected_artifact_count} selected artifact(s)." }
                        }
                    }
                    span { class: "shrink-0", "Enter to send" }
                }
            }
        }
    }
}
