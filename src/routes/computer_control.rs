use dioxus::prelude::*;
use crate::state::use_global_state;

mod execution_log; mod control_center; mod supervisor_panel; mod supervisor_intervention; mod voice_header;
mod interaction_log; mod widgets; mod widgets_panel; mod header; mod single_widget;

#[component]
pub fn ComputerControl() -> Element {
    let state = use_global_state();
    crate::routes::listening::speech::use_speech_recognition(state);
    let db = state.read().db.clone();
    
    let mut settings = use_resource({
        let db = db.clone();
        move || {
            let db = db.clone();
            async move { crate::backend::get_all_settings(&db).await }
        }
    });
    let values = settings.read().cloned().unwrap_or_default();
    let pending_action = values.get("computer_pending_action").cloned().unwrap_or_default();

    let run_data = use_resource({
        let db = db.clone();
        move || {
            let db = db.clone();
            async move {
                let run = crate::backend::get_latest_execution_run(&db).await.ok().flatten();
                let evs = match &run {
                    Some(r) => crate::backend::get_execution_events(&db, r.id).await.unwrap_or_default(),
                    None => vec![],
                };
                (run, evs)
            }
        }
    });
    let (latest_run, events) = run_data.read().cloned().unwrap_or_default();

    let mut run_data_poll = run_data.clone();
    let mut settings_poll = settings.clone();
    let run_active = latest_run.as_ref().map_or(false, |r| r.status == "running");
    
    use_future(move || async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(if run_active { 1 } else { 3 })).await;
            run_data_poll.restart();
            settings_poll.restart();
        }
    });

    let active_widgets = use_signal(|| vec![
        "Interaction Log".to_string(), "Control Center".to_string(), "System Load".to_string(),
        "GPU Performance".to_string(), "Background Processes".to_string(), "Environment".to_string(),
    ]);
    let show_customizer = use_signal(|| false);

    let active_list = active_widgets.read().clone();
    let (has_log, has_center, has_load, has_gpu, has_bg, has_env) = (
        active_list.contains(&"Interaction Log".to_string()),
        active_list.contains(&"Control Center".to_string()),
        active_list.contains(&"System Load".to_string()),
        active_list.contains(&"GPU Performance".to_string()),
        active_list.contains(&"Background Processes".to_string()),
        active_list.contains(&"Environment".to_string()),
    );
    let is_listening = *state.read().is_listening.read();
    let mut is_listening_sig = state.peek().is_listening;
    let mut is_inhibited_sig = state.peek().is_inhibited;

    rsx! {
        div { class: "flex-1 flex flex-col h-screen bg-white text-slate-800 overflow-hidden font-sans relative",
            header::HeaderBar { active_widgets, show_customizer }
            voice_header::VoiceHeader {
                is_listening,
                on_terminate: move |_| {
                    is_listening_sig.set(false);
                    is_inhibited_sig.set(false);
                }
            }
            div { class: "flex-1 flex flex-col lg:flex-row overflow-y-auto lg:overflow-hidden min-h-0",
                if active_list.len() == 1 {
                    single_widget::SingleWidgetLayout { has_log, has_center, has_load, has_gpu, has_bg, has_env, is_listening, run_status: latest_run.as_ref().map(|r| r.status.clone()).unwrap_or_else(|| "idle".to_string()), events, settings }
                } else {
                    if has_log { interaction_log::InteractionLog { events: events.clone() } }
                    if has_center { control_center::ControlCenter { run_status: latest_run.as_ref().map(|r| r.status.clone()).unwrap_or_else(|| "idle".to_string()), events: events.clone() } }
                    if !pending_action.is_empty() {
                        supervisor_panel::SupervisorPanel { db: db.clone(), pending_action: pending_action.clone(), pending_tool: "".to_string(), approval_state: "".to_string(), on_refresh: move || { settings.restart(); } }
                    } else if has_load || has_gpu || has_bg || has_env {
                        widgets_panel::WidgetsPanel { db: db.clone(), has_load, has_gpu, has_bg, has_env, pending_action: pending_action.clone(), on_refresh: move || { settings.restart(); } }
                    }
                }
            }
        }
    }
}
