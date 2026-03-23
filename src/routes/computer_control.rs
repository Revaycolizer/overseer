use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

#[component]
pub fn ComputerControl() -> Element {
    let state = crate::state::use_global_state();
    let db = state.read().db.clone();
    let settings_db = db.clone();
    let supervisors_db = db.clone();

    let mut settings = use_resource(move || {
        let db = settings_db.clone();
        async move { crate::backend::get_all_settings(&db).await }
    });

    let supervisors = use_resource(move || {
        let db = supervisors_db.clone();
        async move { crate::backend::get_supervisors(&db).await.unwrap_or_default() }
    });

    let values = settings.read().cloned().unwrap_or_default();
    let computer_supervisor = supervisors
        .read()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .find(|supervisor| supervisor.domain == "computer");

    let supervision_mode = values
        .get("computer_supervision_mode")
        .cloned()
        .unwrap_or_else(|| "approval_required".to_string());
    let max_actions = values
        .get("computer_supervision_max_actions")
        .cloned()
        .unwrap_or_else(|| "25".to_string());
    let session_timeout_ms = values
        .get("computer_supervision_session_timeout_ms")
        .cloned()
        .unwrap_or_else(|| "600000".to_string());
    let allowlist = values
        .get("computer_supervision_allowlist")
        .cloned()
        .unwrap_or_default();
    let session_state = values
        .get("computer_session_state")
        .cloned()
        .unwrap_or_else(|| "idle".to_string());
    let session_note = values
        .get("computer_session_note")
        .cloned()
        .unwrap_or_else(|| "No supervised computer session is active.".to_string());
    let approval_state = values
        .get("computer_approval_state")
        .cloned()
        .unwrap_or_else(|| "idle".to_string());
    let pending_tool = values
        .get("computer_pending_tool")
        .cloned()
        .unwrap_or_default();
    let pending_action = values
        .get("computer_pending_action")
        .cloned()
        .unwrap_or_default();
    let action_count = values
        .get("computer_session_action_count")
        .cloned()
        .unwrap_or_else(|| "0".to_string());
    let approval_label = approval_state.replace('_', " ");
    let pending_tool_label = if pending_tool.is_empty() {
        "None".to_string()
    } else {
        pending_tool.clone()
    };

    rsx! {
        div { class: "flex-1 flex flex-col min-h-0 bg-slate-50 overflow-y-auto custom-scrollbar",
            main { class: "flex-1 p-6 md:p-10 max-w-6xl mx-auto w-full space-y-8",
                div { class: "flex flex-col lg:flex-row lg:items-start lg:justify-between gap-6",
                    div { class: "space-y-3",
                        p { class: "text-[10px] font-black uppercase tracking-[0.24em] text-slate-400", "Computer Supervision" }
                        h1 { class: "text-4xl font-black tracking-tight text-slate-900", "Supervise a whole-computer session safely." }
                        p { class: "text-base text-slate-600 max-w-3xl leading-relaxed",
                            "This control surface lets Overseer operate at the computer level with explicit guardrails. Start in observation or approval mode, cap actions, restrict allowed apps, and keep the full run trace attached."
                        }
                    }
                    div { class: "flex flex-wrap gap-3 items-center",
                        SessionStateBadge { state: session_state.clone() }
                        button {
                            class: "inline-flex items-center gap-2 self-start rounded-xl border border-slate-200 bg-white px-4 py-2 text-sm font-semibold text-slate-700 shadow-sm hover:bg-slate-50 transition-all",
                            onclick: move |_| settings.restart(),
                            MaterialIcon { name: "refresh", size: 18 }
                            "Refresh"
                        }
                    }
                }

                div { class: "grid grid-cols-1 xl:grid-cols-[minmax(0,1fr)_340px] gap-8",
                    div { class: "space-y-8",
                        div { class: "bg-white rounded-3xl border border-slate-200 shadow-sm p-6 space-y-6",
                            div { class: "flex items-center justify-between gap-4",
                                div {
                                    h2 { class: "text-lg font-black text-slate-900", "Session Guardrails" }
                                    p { class: "text-sm text-slate-500 mt-1", "Choose how much autonomy Overseer gets before touching the desktop." }
                                }
                                span { class: "text-[10px] font-black uppercase tracking-[0.18em] px-3 py-1 rounded-full bg-slate-100 text-slate-500",
                                    "Policy Backed"
                                }
                            }

                            div { class: "grid gap-6 md:grid-cols-2",
                                div { class: "space-y-2",
                                    label { class: "text-sm font-bold text-slate-900", "Supervision Mode" }
                                    select {
                                        class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700",
                                        value: "{supervision_mode}",
                                        onchange: {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            move |evt| {
                                                let db = db.clone();
                                                let mut settings = settings.clone();
                                                spawn(async move {
                                                    let _ = crate::backend::set_setting(&db, "computer_supervision_mode", &evt.value()).await;
                                                    settings.restart();
                                                });
                                            }
                                        },
                                        option { value: "observe_only", "Observe Only" }
                                        option { value: "approval_required", "Approval Required" }
                                        option { value: "session_trusted", "Trusted Per Session" }
                                        option { value: "trusted_auto_run", "Trusted Auto-Run" }
                                    }
                                    p { class: "text-xs text-slate-500", "Start with approval mode until the operator surface is fully trusted." }
                                }

                                div { class: "space-y-2",
                                    label { class: "text-sm font-bold text-slate-900", "Maximum Actions Per Session" }
                                    input {
                                        r#type: "number",
                                        class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700",
                                        value: "{max_actions}",
                                        min: "1",
                                        max: "200",
                                        onchange: {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            move |evt| {
                                                let db = db.clone();
                                                let mut settings = settings.clone();
                                                spawn(async move {
                                                    let _ = crate::backend::set_setting(&db, "computer_supervision_max_actions", &evt.value()).await;
                                                    settings.restart();
                                                });
                                            }
                                        },
                                    }
                                    p { class: "text-xs text-slate-500", "Prevents long runaway sessions on the desktop." }
                                }

                                div { class: "space-y-2",
                                    label { class: "text-sm font-bold text-slate-900", "Session Timeout (ms)" }
                                    input {
                                        r#type: "number",
                                        class: "w-full bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700",
                                        value: "{session_timeout_ms}",
                                        min: "10000",
                                        step: "10000",
                                        onchange: {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            move |evt| {
                                                let db = db.clone();
                                                let mut settings = settings.clone();
                                                spawn(async move {
                                                    let _ = crate::backend::set_setting(&db, "computer_supervision_session_timeout_ms", &evt.value()).await;
                                                    settings.restart();
                                                });
                                            }
                                        },
                                    }
                                    p { class: "text-xs text-slate-500", "Automatic stop window for broad computer-control sessions." }
                                }

                                div { class: "space-y-2",
                                    label { class: "text-sm font-bold text-slate-900", "Allowed Apps / Windows" }
                                    textarea {
                                        class: "w-full min-h-[120px] bg-slate-50 border border-slate-200 rounded-2xl p-4 text-sm text-slate-700",
                                        value: "{allowlist}",
                                        placeholder: "One app, title keyword, or process per line...",
                                        oninput: {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            move |evt| {
                                                let db = db.clone();
                                                let mut settings = settings.clone();
                                                spawn(async move {
                                                    let _ = crate::backend::set_setting(&db, "computer_supervision_allowlist", &evt.value()).await;
                                                    settings.restart();
                                                });
                                            }
                                        },
                                    }
                                    p { class: "text-xs text-slate-500", "Use one line per app or keyword, for example `chrome.exe` or `Visual Studio Code`." }
                                }
                            }
                        }

                        div { class: "bg-white rounded-3xl border border-slate-200 shadow-sm p-6 space-y-5",
                            div {
                                h2 { class: "text-lg font-black text-slate-900", "Session Controls" }
                                p { class: "text-sm text-slate-500 mt-1", "These controls manage the operator state. The actual desktop actions should still remain approval-aware and fully traced." }
                            }
                            div { class: "rounded-2xl border border-slate-200 bg-slate-50 p-4",
                                p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "Current Session Note" }
                                p { class: "text-sm text-slate-700 mt-2 whitespace-pre-wrap", "{session_note}" }
                            }
                            div { class: "grid gap-4 md:grid-cols-3",
                                div { class: "rounded-2xl border border-slate-200 bg-slate-50 p-4",
                                    p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "Actions Used" }
                                    p { class: "text-2xl font-black text-slate-900 mt-2", "{action_count}" }
                                }
                                div { class: "rounded-2xl border border-slate-200 bg-slate-50 p-4",
                                    p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "Approval State" }
                                    p { class: "text-2xl font-black text-slate-900 mt-2", "{approval_label}" }
                                }
                                div { class: "rounded-2xl border border-slate-200 bg-slate-50 p-4",
                                    p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "Pending Tool" }
                                    p { class: "text-sm font-bold text-slate-900 mt-2 break-all", "{pending_tool_label}" }
                                }
                            }

                            if !pending_action.is_empty() {
                                div { class: "rounded-2xl border border-blue-200 bg-blue-50 p-4 space-y-4",
                                    div {
                                        p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-blue-500", "Pending Approval" }
                                        p { class: "text-sm text-blue-900 mt-2 whitespace-pre-wrap", "{pending_action}" }
                                    }
                                    div { class: "flex flex-wrap gap-3",
                                        button {
                                            class: "inline-flex items-center gap-2 rounded-xl bg-blue-600 text-white px-4 py-3 text-sm font-bold hover:bg-blue-700 transition-all",
                                            onclick: {
                                                let db = db.clone();
                                                let mut settings = settings.clone();
                                                move |_| {
                                                    let db = db.clone();
                                                    let mut settings = settings.clone();
                                                    spawn(async move {
                                                        let _ = crate::backend::approve_pending_computer_action(&db).await;
                                                        settings.restart();
                                                    });
                                                }
                                            },
                                            MaterialIcon { name: "task_alt", size: 18, color: MaterialIconColor::Light }
                                            "Approve Next Action"
                                        }
                                        button {
                                            class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white text-slate-700 px-4 py-3 text-sm font-bold hover:bg-slate-50 transition-all",
                                            onclick: {
                                                let db = db.clone();
                                                let mut settings = settings.clone();
                                                move |_| {
                                                    let db = db.clone();
                                                    let mut settings = settings.clone();
                                                    spawn(async move {
                                                        let _ = crate::backend::reject_pending_computer_action(&db).await;
                                                        settings.restart();
                                                    });
                                                }
                                            },
                                            MaterialIcon { name: "cancel", size: 18 }
                                            "Reject Pending Action"
                                        }
                                    }
                                }
                            }

                            div { class: "flex flex-wrap gap-3",
                                button {
                                    class: "inline-flex items-center gap-2 rounded-xl bg-slate-900 text-white px-4 py-3 text-sm font-bold hover:bg-slate-800 transition-all",
                                    onclick: {
                                        let db = db.clone();
                                        let mut settings = settings.clone();
                                        move |_| {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            spawn(async move {
                                                let _ = crate::backend::activate_computer_session(
                                                    &db,
                                                    "observing",
                                                    "Observation mode enabled. Overseer should inspect the desktop state and propose actions without executing them.",
                                                )
                                                .await;
                                                settings.restart();
                                            });
                                        }
                                    },
                                    MaterialIcon { name: "visibility", size: 18, color: MaterialIconColor::Light }
                                    "Observe"
                                }
                                button {
                                    class: "inline-flex items-center gap-2 rounded-xl bg-blue-600 text-white px-4 py-3 text-sm font-bold hover:bg-blue-700 transition-all",
                                    onclick: {
                                        let db = db.clone();
                                        let mut settings = settings.clone();
                                        move |_| {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            spawn(async move {
                                                let _ = crate::backend::activate_computer_session(
                                                    &db,
                                                    "approval_session",
                                                    "Approval-required session started. Overseer may plan and request permission before each desktop action.",
                                                )
                                                .await;
                                                settings.restart();
                                            });
                                        }
                                    },
                                    MaterialIcon { name: "fact_check", size: 18, color: MaterialIconColor::Light }
                                    "Start Approval Session"
                                }
                                button {
                                    class: "inline-flex items-center gap-2 rounded-xl bg-emerald-600 text-white px-4 py-3 text-sm font-bold hover:bg-emerald-700 transition-all",
                                    onclick: {
                                        let db = db.clone();
                                        let mut settings = settings.clone();
                                        move |_| {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            spawn(async move {
                                                let _ = crate::backend::activate_computer_session(
                                                    &db,
                                                    "trusted_session",
                                                    "Trusted computer session started. Overseer may execute bounded desktop actions inside the configured policy guardrails.",
                                                )
                                                .await;
                                                settings.restart();
                                            });
                                        }
                                    },
                                    MaterialIcon { name: "verified_user", size: 18, color: MaterialIconColor::Light }
                                    "Start Trusted Session"
                                }
                                button {
                                    class: "inline-flex items-center gap-2 rounded-xl border border-amber-200 bg-amber-50 text-amber-800 px-4 py-3 text-sm font-bold hover:bg-amber-100 transition-all",
                                    onclick: {
                                        let db = db.clone();
                                        let mut settings = settings.clone();
                                        move |_| {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            spawn(async move {
                                                let _ = crate::backend::set_setting(&db, "computer_session_state", "paused").await;
                                                let _ = crate::backend::set_setting(&db, "computer_session_note", "Computer supervision paused. No further desktop actions should execute until resumed.").await;
                                                let _ = crate::backend::clear_computer_pending_action(&db).await;
                                                settings.restart();
                                            });
                                        }
                                    },
                                    MaterialIcon { name: "pause_circle", size: 18, color: MaterialIconColor::Custom("#92400e".into()) }
                                    "Pause"
                                }
                                button {
                                    class: "inline-flex items-center gap-2 rounded-xl border border-red-200 bg-red-50 text-red-700 px-4 py-3 text-sm font-bold hover:bg-red-100 transition-all",
                                    onclick: {
                                        let db = db.clone();
                                        let mut settings = settings.clone();
                                        move |_| {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            spawn(async move {
                                                let _ = crate::backend::set_setting(&db, "computer_session_state", "emergency_stopped").await;
                                                let _ = crate::backend::set_setting(&db, "computer_session_note", "Emergency stop engaged. Overseer should not take any desktop action until the session is reset.").await;
                                                let _ = crate::backend::clear_computer_pending_action(&db).await;
                                                settings.restart();
                                            });
                                        }
                                    },
                                    MaterialIcon { name: "emergency_home", size: 18, color: MaterialIconColor::Custom("#dc2626".into()) }
                                    "Emergency Stop"
                                }
                                button {
                                    class: "inline-flex items-center gap-2 rounded-xl border border-slate-200 bg-white text-slate-700 px-4 py-3 text-sm font-bold hover:bg-slate-50 transition-all",
                                    onclick: {
                                        let db = db.clone();
                                        let mut settings = settings.clone();
                                        move |_| {
                                            let db = db.clone();
                                            let mut settings = settings.clone();
                                            spawn(async move {
                                                let _ = crate::backend::activate_computer_session(
                                                    &db,
                                                    "idle",
                                                    "No supervised computer session is active.",
                                                )
                                                .await;
                                                settings.restart();
                                            });
                                        }
                                    },
                                    MaterialIcon { name: "stop_circle", size: 18 }
                                    "End Session"
                                }
                            }
                        }
                    }

                    div { class: "space-y-6",
                        div { class: "bg-white rounded-3xl border border-slate-200 shadow-sm p-6 space-y-4",
                            h2 { class: "text-lg font-black text-slate-900", "Operator Stack" }
                            if let Some(supervisor) = computer_supervisor {
                                {
                                    let description = supervisor.description.clone().unwrap_or_default();
                                    let model = supervisor
                                        .model
                                        .clone()
                                        .unwrap_or_else(|| "default runtime".to_string());
                                    rsx! {
                                        div { class: "rounded-2xl border border-slate-200 bg-slate-50 p-4 space-y-3",
                                            div { class: "flex items-center justify-between gap-3",
                                                h3 { class: "text-sm font-black text-slate-900", "{supervisor.name}" }
                                                span { class: "text-[10px] font-black uppercase tracking-[0.18em] px-2 py-1 rounded-full bg-blue-50 text-blue-700", "computer" }
                                            }
                                            p { class: "text-sm text-slate-600", "{description}" }
                                            div { class: "space-y-2 text-xs text-slate-500",
                                                p { "Model: {model}" }
                                                p {
                                                    "Bypass synthesizer: "
                                                    {if supervisor.bypass_synthesizer { "enabled" } else { "disabled" }}
                                                }
                                            }
                                        }
                                    }
                                }
                            } else {
                                div { class: "rounded-2xl border border-dashed border-slate-200 bg-slate-50 p-4 text-sm text-slate-500",
                                    "Computer Supervisor has not been seeded yet. Restart the app once to let Overseer ensure the new supervisor is present."
                                }
                            }
                        }

                        div { class: "bg-white rounded-3xl border border-slate-200 shadow-sm p-6 space-y-4",
                            h2 { class: "text-lg font-black text-slate-900", "Safety Notes" }
                            ul { class: "space-y-3 text-sm text-slate-600 leading-relaxed",
                                li { "Observe first. Let Overseer see and explain the next action before it gets a trusted session." }
                                li { "Keep the allowlist narrow. Restrict the agent to specific apps or window titles." }
                                li { "Treat emergency stop as the kill switch. If confidence drops or behavior looks wrong, stop the session immediately." }
                                li { "Use the execution trace after every run. Broad desktop actions should always be auditable." }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SessionStateBadge(state: String) -> Element {
    let (class, label) = match state.as_str() {
        "observing" => ("bg-slate-100 text-slate-700", "Observing"),
        "approval_session" => ("bg-blue-50 text-blue-700", "Approval Session"),
        "paused" => ("bg-amber-50 text-amber-700", "Paused"),
        "emergency_stopped" => ("bg-red-50 text-red-700", "Emergency Stop"),
        "trusted_session" => ("bg-emerald-50 text-emerald-700", "Trusted Session"),
        _ => ("bg-slate-100 text-slate-500", "Idle"),
    };

    rsx! {
        span { class: format!("inline-flex items-center rounded-full px-3 py-1 text-[10px] font-black uppercase tracking-[0.18em] {}", class),
            "{label}"
        }
    }
}
