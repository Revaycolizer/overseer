use dioxus::prelude::*;
use dioxus_material_icons::{MaterialIcon, MaterialIconColor};

use crate::backend::{self, Workflow, WorkflowRun, WorkflowStep};
use crate::routes::execution_trace::ExecutionTracePanel;
use crate::state::use_global_state;

#[component]
pub fn WorkflowAutomation() -> Element {
    let state = use_global_state();
    let db = state.read().db.clone();

    let db_for_workflows = db.clone();
    let workflows_resource = use_resource(move || {
        let db = db_for_workflows.clone();
        async move { backend::get_workflows(&db).await.unwrap_or_default() }
    });

    let workflows = workflows_resource.read().cloned().unwrap_or_default();
    let mut selected_workflow_id = use_signal(|| workflows.first().map(|w| w.id));
    let run_feedback = use_signal(String::new);
    let is_running = use_signal(|| false);

    let workflow_list_for_effect = workflows.clone();
    use_effect(move || {
        let workflow_list = workflow_list_for_effect.clone();
        if selected_workflow_id().is_none() {
            if let Some(first) = workflow_list.first() {
                selected_workflow_id.set(Some(first.id));
            }
        }
    });

    let db_for_steps = db.clone();
    let steps_resource = use_resource(move || {
        let db = db_for_steps.clone();
        let workflow_id = selected_workflow_id();
        async move {
            if let Some(workflow_id) = workflow_id {
                backend::get_workflow_steps(&db, workflow_id)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let db_for_runs = db.clone();
    let runs_resource = use_resource(move || {
        let db = db_for_runs.clone();
        let workflow_id = selected_workflow_id();
        async move {
            if let Some(workflow_id) = workflow_id {
                backend::get_workflow_runs(&db, workflow_id)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let selected_workflow = workflows
        .iter()
        .find(|workflow| Some(workflow.id) == selected_workflow_id())
        .cloned();

    rsx! {
        main { class: "flex-1 min-h-0 bg-[#F7FAFC] overflow-hidden",
            div { class: "h-full grid grid-cols-1 xl:grid-cols-[320px_minmax(0,1fr)]",
                div { class: "border-r border-slate-200 bg-white overflow-y-auto custom-scrollbar p-5",
                    div { class: "flex items-end justify-between gap-4 mb-6",
                        div {
                            h2 { class: "text-2xl font-extrabold tracking-tight text-slate-900", "Workflow Automation" }
                            p { class: "text-sm text-slate-500 mt-1", "Persist reusable multi-step runs and replay them on demand." }
                        }
                        button {
                            class: "bg-slate-900 text-white px-3 py-2 rounded-xl text-xs font-bold hover:bg-slate-800 transition-all flex items-center gap-2",
                            onclick: {
                                let db = db.clone();
                                let workflows_resource = workflows_resource.clone();
                                move |_| {
                                    let db = db.clone();
                                    let mut workflows_resource = workflows_resource.clone();
                                    spawn(async move {
                                        if let Ok(id) = backend::add_workflow(&db, "New Workflow").await {
                                            selected_workflow_id.set(Some(id));
                                            workflows_resource.restart();
                                        }
                                    });
                                }
                            },
                            MaterialIcon { name: "add", size: 18, color: MaterialIconColor::Light }
                            "New"
                        }
                    }

                    div { class: "space-y-3",
                        if workflows.is_empty() {
                            div { class: "rounded-2xl border border-dashed border-slate-200 p-8 text-center text-sm text-slate-400",
                                "No workflows yet. Create one to start building repeatable automations."
                            }
                        } else {
                            for workflow in workflows.clone() {
                                div {
                                    key: "{workflow.id}",
                                    class: format!(
                                        "rounded-2xl border p-4 cursor-pointer transition-all {}",
                                        if Some(workflow.id) == selected_workflow_id() {
                                            "border-blue-300 bg-blue-50/60 shadow-sm"
                                        } else {
                                            "border-slate-200 bg-white hover:border-slate-300"
                                        }
                                    ),
                                    onclick: {
                                        let workflow_id = workflow.id;
                                        move |_| selected_workflow_id.set(Some(workflow_id))
                                    },
                                    div { class: "flex items-start justify-between gap-3",
                                        div {
                                            h3 { class: "font-bold text-slate-900", "{workflow.name}" }
                                            p { class: "text-xs text-slate-500 mt-1 line-clamp-2", "{workflow.description.clone().unwrap_or_default()}" }
                                        }
                                        span { class: "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full bg-slate-100 text-slate-500",
                                            "{workflow.status}"
                                        }
                                    }
                                    div { class: "mt-3 flex items-center justify-between text-[11px] text-slate-400",
                                        span { "{workflow.trigger_type}" }
                                        button {
                                            class: "hover:text-red-500 transition-colors",
                                            onclick: {
                                                let workflow_id = workflow.id;
                                                let db = db.clone();
                                                let workflows_resource = workflows_resource.clone();
                                                move |evt| {
                                                    evt.stop_propagation();
                                                    let db = db.clone();
                                                    let mut workflows_resource = workflows_resource.clone();
                                                    spawn(async move {
                                                        let _ = backend::delete_workflow(&db, workflow_id).await;
                                                        if selected_workflow_id() == Some(workflow_id) {
                                                            selected_workflow_id.set(None);
                                                        }
                                                        workflows_resource.restart();
                                                    });
                                                }
                                            },
                                            MaterialIcon { name: "delete", size: 16 }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { class: "min-w-0 flex flex-col overflow-hidden",
                    if let Some(workflow) = selected_workflow {
                        WorkflowEditor {
                            workflow,
                            steps_resource: steps_resource.clone(),
                            runs_resource: runs_resource.clone(),
                            workflows_resource: workflows_resource.clone(),
                            run_feedback,
                            is_running,
                        }
                    } else {
                        div { class: "h-full flex flex-col items-center justify-center text-center text-slate-400 p-12",
                            div { class: "w-24 h-24 rounded-full bg-white border border-slate-200 shadow-sm flex items-center justify-center mb-6",
                                MaterialIcon { name: "account_tree", size: 42, color: MaterialIconColor::Custom("#94a3b8".into()) }
                            }
                            h3 { class: "text-xl font-bold text-slate-900 mb-2", "Select a workflow" }
                            p { class: "max-w-md", "Choose a workflow from the left or create a new one to start composing multi-step automation runs." }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn WorkflowEditor(
    workflow: Workflow,
    mut steps_resource: Resource<Vec<WorkflowStep>>,
    mut runs_resource: Resource<Vec<WorkflowRun>>,
    mut workflows_resource: Resource<Vec<Workflow>>,
    mut run_feedback: Signal<String>,
    mut is_running: Signal<bool>,
) -> Element {
    let state = use_global_state();
    let db = state.read().db.clone();
    let agent_system = state.read().agent_system.clone();
    let steps = steps_resource.read().cloned().unwrap_or_default();
    let runs = runs_resource.read().cloned().unwrap_or_default();
    let mut selected_workflow_run_id = use_signal(|| runs.first().map(|run| run.id));

    let runs_for_effect = runs.clone();
    use_effect(move || {
        let current = selected_workflow_run_id();
        if runs_for_effect.is_empty() {
            if current.is_some() {
                selected_workflow_run_id.set(None);
            }
        } else if !runs_for_effect.iter().any(|run| Some(run.id) == current) {
            selected_workflow_run_id.set(Some(runs_for_effect[0].id));
        }
    });

    let db_for_trace_runs = db.clone();
    let trace_runs_resource = use_resource(move || {
        let db = db_for_trace_runs.clone();
        let workflow_run_id = selected_workflow_run_id();
        async move {
            if let Some(id) = workflow_run_id {
                backend::get_execution_runs_for_workflow_run(&db, id, 10)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let trace_runs = trace_runs_resource.read().cloned().unwrap_or_default();
    let mut selected_execution_run_id = use_signal(|| trace_runs.first().map(|run| run.id));

    let trace_runs_for_effect = trace_runs.clone();
    use_effect(move || {
        let current = selected_execution_run_id();
        if trace_runs_for_effect.is_empty() {
            if current.is_some() {
                selected_execution_run_id.set(None);
            }
        } else if !trace_runs_for_effect
            .iter()
            .any(|run| Some(run.id) == current)
        {
            selected_execution_run_id.set(Some(trace_runs_for_effect[0].id));
        }
    });

    let db_for_trace_events = db.clone();
    let trace_events_resource = use_resource(move || {
        let db = db_for_trace_events.clone();
        let run_id = selected_execution_run_id();
        async move {
            if let Some(id) = run_id {
                backend::get_execution_events(&db, id)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let trace_events = trace_events_resource.read().cloned().unwrap_or_default();
    let trace_runs_loading = trace_runs_resource.read().is_none();
    let trace_events_loading = trace_events_resource.read().is_none();
    let enabled_step_count = steps.iter().filter(|step| step.enabled).count();
    let latest_run = runs.first().cloned();
    let selected_run = runs
        .iter()
        .find(|run| Some(run.id) == selected_workflow_run_id())
        .cloned();

    let db_for_step_runs = db.clone();
    let step_runs_resource = use_resource(move || {
        let db = db_for_step_runs.clone();
        let workflow_run_id = selected_workflow_run_id();
        async move {
            if let Some(id) = workflow_run_id {
                backend::get_workflow_step_runs(&db, id)
                    .await
                    .unwrap_or_default()
            } else {
                vec![]
            }
        }
    });

    let step_runs = step_runs_resource.read().cloned().unwrap_or_default();

    rsx! {
        div { class: "flex-1 overflow-y-auto custom-scrollbar p-8 space-y-8",
            div { class: "bg-white rounded-3xl border border-slate-200 shadow-sm p-8 space-y-6",
                div { class: "flex flex-col lg:flex-row lg:items-start justify-between gap-6",
                    div { class: "flex-1 space-y-4",
                        input {
                            class: "w-full text-3xl font-black tracking-tight text-slate-900 bg-transparent border-none focus:ring-0 p-0",
                            value: "{workflow.name}",
                            oninput: {
                                let db = db.clone();
                                let workflows_resource = workflows_resource.clone();
                                let workflow_for_name = workflow.clone();
                                move |evt| {
                                    let mut updated = workflow_for_name.clone();
                                    updated.name = evt.value();
                                    let db = db.clone();
                                    let mut workflows_resource = workflows_resource.clone();
                                    spawn(async move {
                                        let _ = backend::update_workflow(&db, updated).await;
                                        workflows_resource.restart();
                                    });
                                }
                            }
                        }
                        textarea {
                            class: "w-full min-h-[80px] bg-slate-50 border border-slate-200 rounded-2xl p-4 text-sm text-slate-600 focus:ring-2 focus:ring-blue-500/10 focus:border-blue-500 transition-all",
                            value: "{workflow.description.clone().unwrap_or_default()}",
                            placeholder: "Describe what this workflow should achieve...",
                            oninput: {
                                let db = db.clone();
                                let workflows_resource = workflows_resource.clone();
                                let workflow_for_description = workflow.clone();
                                move |evt| {
                                    let mut updated = workflow_for_description.clone();
                                    updated.description = if evt.value().trim().is_empty() { None } else { Some(evt.value()) };
                                    let db = db.clone();
                                    let mut workflows_resource = workflows_resource.clone();
                                    spawn(async move {
                                        let _ = backend::update_workflow(&db, updated).await;
                                        workflows_resource.restart();
                                    });
                                }
                            }
                        }
                    }

                    div { class: "flex flex-col gap-3 lg:w-[260px]",
                        select {
                            class: "bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700",
                            value: "{workflow.trigger_type}",
                            onchange: {
                                let db = db.clone();
                                let workflows_resource = workflows_resource.clone();
                                let workflow_for_trigger = workflow.clone();
                                move |evt| {
                                    let mut updated = workflow_for_trigger.clone();
                                    updated.trigger_type = evt.value();
                                    let db = db.clone();
                                    let mut workflows_resource = workflows_resource.clone();
                                    spawn(async move {
                                        let _ = backend::update_workflow(&db, updated).await;
                                        workflows_resource.restart();
                                    });
                                }
                            },
                            option { value: "manual", "Manual Trigger" }
                            option { value: "scheduled", "Scheduled Trigger" }
                            option { value: "event", "Event Trigger" }
                        }
                        select {
                            class: "bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-semibold text-slate-700",
                            value: "{workflow.status}",
                            onchange: {
                                let db = db.clone();
                                let workflows_resource = workflows_resource.clone();
                                let workflow_for_status = workflow.clone();
                                move |evt| {
                                    let mut updated = workflow_for_status.clone();
                                    updated.status = evt.value();
                                    let db = db.clone();
                                    let mut workflows_resource = workflows_resource.clone();
                                    spawn(async move {
                                        let _ = backend::update_workflow(&db, updated).await;
                                        workflows_resource.restart();
                                    });
                                }
                            },
                            option { value: "draft", "Draft" }
                            option { value: "active", "Active" }
                            option { value: "paused", "Paused" }
                        }
                        button {
                            disabled: is_running(),
                            class: format!(
                                "px-4 py-3 rounded-xl text-sm font-bold transition-all flex items-center justify-center gap-2 shadow-sm {}",
                                if is_running() {
                                    "bg-slate-200 text-slate-400 cursor-not-allowed shadow-none"
                                } else {
                                    "bg-blue-600 text-white hover:bg-blue-700"
                                }
                            ),
                            onclick: {
                                let db = db.clone();
                                let agent_system = agent_system.clone();
                                let runs_resource = runs_resource.clone();
                                move |_| {
                                    let db = db.clone();
                                    let agent_system = agent_system.clone();
                                    let mut runs_resource = runs_resource.clone();
                                    is_running.set(true);
                                    run_feedback.set("Running workflow...".to_string());
                                    spawn(async move {
                                        match backend::run_workflow(&db, &agent_system, workflow.id).await {
                                            Ok(run_id) => {
                                                run_feedback.set(format!("Workflow run {} completed.", run_id));
                                                runs_resource.restart();
                                            }
                                            Err(err) => {
                                                run_feedback.set(format!("Workflow run failed: {}", err));
                                            }
                                        }
                                        is_running.set(false);
                                    });
                                }
                            },
                            MaterialIcon { name: "play_arrow", size: 18, color: MaterialIconColor::Light }
                            if is_running() { "Running..." } else if runs.is_empty() { "Run Workflow" } else { "Run Again" }
                        }
                        button {
                            class: "bg-slate-900 text-white px-4 py-3 rounded-xl text-sm font-bold hover:bg-slate-800 transition-all flex items-center justify-center gap-2 shadow-sm",
                            onclick: {
                                let db = db.clone();
                                let steps_resource = steps_resource.clone();
                                move |_| {
                                    let db = db.clone();
                                    let mut steps_resource = steps_resource.clone();
                                    spawn(async move {
                                        let _ = backend::add_workflow_step(&db, workflow.id).await;
                                        steps_resource.restart();
                                    });
                                }
                            },
                            MaterialIcon { name: "add", size: 18, color: MaterialIconColor::Light }
                            "Add Step"
                        }
                    }
                }

                div { class: "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-4 gap-3",
                    WorkflowMetricCard {
                        label: "Enabled Steps".to_string(),
                        value: enabled_step_count.to_string(),
                        tone: "slate".to_string(),
                    }
                    WorkflowMetricCard {
                        label: "Trigger".to_string(),
                        value: prettify_workflow_token(&workflow.trigger_type),
                        tone: "blue".to_string(),
                    }
                    WorkflowMetricCard {
                        label: "Workflow State".to_string(),
                        value: prettify_workflow_token(&workflow.status),
                        tone: workflow_status_tone(&workflow.status).to_string(),
                    }
                    WorkflowMetricCard {
                        label: "Latest Run".to_string(),
                        value: latest_run
                            .as_ref()
                            .map(|run| prettify_workflow_token(&run.status))
                            .unwrap_or_else(|| "No Runs".to_string()),
                        tone: latest_run
                            .as_ref()
                            .map(|run| workflow_status_tone(&run.status).to_string())
                            .unwrap_or_else(|| "slate".to_string()),
                    }
                }

                if !run_feedback().is_empty() {
                    div { class: format!(
                        "rounded-2xl px-4 py-3 text-sm font-medium border {}",
                        if run_feedback().to_lowercase().contains("failed") {
                            "border-red-200 bg-red-50 text-red-700"
                        } else {
                            "border-blue-200 bg-blue-50 text-blue-700"
                        }
                    ),
                        "{run_feedback}"
                    }
                }
            }

            div { class: "grid grid-cols-1 2xl:grid-cols-[minmax(0,1fr)_400px] gap-8",
                div { class: "space-y-4",
                    h3 { class: "text-sm font-bold uppercase tracking-widest text-slate-400", "Workflow Steps" }
                    if steps.is_empty() {
                        div { class: "rounded-3xl border border-dashed border-slate-200 bg-white p-10 text-center text-sm text-slate-400",
                            "No steps yet. Add a step and describe the prompt or action it should run."
                        }
                    } else {
                        for step in steps {
                            WorkflowStepCard {
                                step,
                                steps_resource: steps_resource.clone(),
                            }
                        }
                    }
                }

                div { class: "space-y-4",
                    h3 { class: "text-sm font-bold uppercase tracking-widest text-slate-400", "Recent Runs" }
                    div { class: "bg-white rounded-3xl border border-slate-200 shadow-sm overflow-hidden",
                        if runs.is_empty() {
                            div { class: "p-8 text-sm text-slate-400 text-center",
                                "No runs yet. Use the run button to execute this workflow."
                            }
                        } else {
                            for run in runs {
                                div {
                                    key: "{run.id}",
                                    class: format!(
                                        "p-4 border-b border-slate-100 last:border-b-0 space-y-2 cursor-pointer transition-colors {}",
                                        if Some(run.id) == selected_workflow_run_id() {
                                            "bg-blue-50/60"
                                        } else {
                                            "hover:bg-slate-50"
                                        }
                                    ),
                                    onclick: {
                                        let run_id = run.id;
                                        move |_| selected_workflow_run_id.set(Some(run_id))
                                    },
                                    div { class: "flex items-center justify-between gap-3",
                                        span { class: format!(
                                            "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                            workflow_status_badge_class(&run.status)
                                        ),
                                            "{prettify_workflow_token(&run.status)}"
                                        }
                                        span { class: "text-[11px] text-slate-400",
                                            {run.created_at.format("%Y-%m-%d %H:%M").to_string()}
                                        }
                                    }
                                    div { class: "flex items-center gap-2",
                                        p { class: "text-xs font-semibold text-slate-700", "Run #{run.id}" }
                                        if Some(run.id) == latest_run.as_ref().map(|latest| latest.id) {
                                            span { class: "text-[10px] font-black uppercase tracking-[0.18em] px-2 py-1 rounded-full bg-slate-100 text-slate-500",
                                                "Latest"
                                            }
                                        }
                                    }
                                    p { class: "text-xs text-slate-500 whitespace-pre-wrap line-clamp-4",
                                        "{run.summary.clone().or(run.error_message.clone()).unwrap_or_else(|| \"No summary yet.\".to_string())}"
                                    }
                                }
                            }
                        }
                    }

                    if let Some(run) = selected_run.clone() {
                        div { class: "bg-white rounded-3xl border border-slate-200 shadow-sm overflow-hidden",
                            div { class: "px-4 py-3 border-b border-slate-100 bg-slate-50/80 flex items-center justify-between gap-3",
                                div {
                                    h4 { class: "text-xs font-black uppercase tracking-[0.24em] text-slate-500", "Selected Run" }
                                    p { class: "text-[11px] text-slate-400 mt-1", "Operational summary for workflow run #{run.id}." }
                                }
                                span { class: format!(
                                    "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                    workflow_status_badge_class(&run.status)
                                ),
                                    "{prettify_workflow_token(&run.status)}"
                                }
                            }
                            div { class: "p-4 space-y-3",
                                WorkflowRunMetaRow {
                                    label: "Started".to_string(),
                                    value: run.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                                }
                                WorkflowRunMetaRow {
                                    label: "Finished".to_string(),
                                    value: run.finished_at
                                        .map(|value| value.format("%Y-%m-%d %H:%M:%S").to_string())
                                        .unwrap_or_else(|| "In progress".to_string()),
                                }
                                WorkflowRunMetaRow {
                                    label: "Conversation".to_string(),
                                    value: run
                                        .conversation_id
                                        .map(|value| value.to_string())
                                        .unwrap_or_else(|| "Not linked".to_string()),
                                }
                                if let Some(summary) = run.summary.clone().filter(|value| !value.trim().is_empty()) {
                                    WorkflowRunSummaryBlock {
                                        label: "Summary".to_string(),
                                        value: summary,
                                    }
                                } else if let Some(error_message) = run.error_message.clone().filter(|value| !value.trim().is_empty()) {
                                    WorkflowRunSummaryBlock {
                                        label: "Failure".to_string(),
                                        value: error_message,
                                    }
                                }
                            }
                        }
                    }

                    if let Some(run_id) = selected_workflow_run_id() {
                        div { class: "bg-white rounded-3xl border border-slate-200 shadow-sm overflow-hidden",
                            div { class: "px-4 py-3 border-b border-slate-100 bg-slate-50/80",
                                h4 { class: "text-xs font-black uppercase tracking-[0.24em] text-slate-500", "Workflow Step Outputs" }
                                p { class: "text-[11px] text-slate-400 mt-1", "Inspect the recorded output for workflow run #{run_id}." }
                            }
                            if step_runs_resource.read().is_none() {
                                div { class: "p-6 text-sm text-slate-400", "Loading step outputs..." }
                            } else if step_runs.is_empty() {
                                div { class: "p-6 text-sm text-slate-400", "No step outputs recorded for this run yet." }
                            } else {
                                div { class: "divide-y divide-slate-100",
                                    for step_run in step_runs {
                                        div {
                                            key: "{step_run.id}",
                                            class: "p-4 space-y-2",
                                            div { class: "flex items-center justify-between gap-3",
                                                p { class: "text-xs font-bold text-slate-700", "Step {step_run.step_order}" }
                                                span { class: format!(
                                                    "text-[10px] font-bold uppercase tracking-widest px-2 py-1 rounded-full {}",
                                                    workflow_status_badge_class(&step_run.status)
                                                ),
                                                    "{prettify_workflow_token(&step_run.status)}"
                                                }
                                            }
                                            if let Some(output) = step_run.output.clone().filter(|value| !value.trim().is_empty()) {
                                                pre { class: "text-[11px] text-slate-600 whitespace-pre-wrap break-words bg-slate-50 border border-slate-200 rounded-2xl p-3 max-h-56 overflow-auto custom-scrollbar",
                                                    "{output}"
                                                }
                                            } else if let Some(error_message) = step_run.error_message.clone().filter(|value| !value.trim().is_empty()) {
                                                pre { class: "text-[11px] text-red-600 whitespace-pre-wrap break-words bg-red-50 border border-red-100 rounded-2xl p-3 max-h-56 overflow-auto custom-scrollbar",
                                                    "{error_message}"
                                                }
                                            } else {
                                                p { class: "text-[11px] text-slate-400", "No output captured." }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    ExecutionTracePanel {
                        heading: "Workflow Trace".to_string(),
                        subtitle: "Inspect the execution runs and event timeline generated by the selected workflow run.".to_string(),
                        empty_title: "No orchestration trace yet".to_string(),
                        empty_body: "Run the workflow and select a run to inspect the planner, supervisor, and tool events behind it.".to_string(),
                        runs: trace_runs,
                        events: trace_events,
                        selected_run_id: selected_execution_run_id,
                        runs_loading: trace_runs_loading,
                        events_loading: trace_events_loading,
                    }
                }
            }
        }
    }
}

fn workflow_status_badge_class(status: &str) -> &'static str {
    match status {
        "completed" | "success" => "bg-emerald-50 text-emerald-600",
        "failed" | "error" => "bg-red-50 text-red-600",
        _ => "bg-blue-50 text-blue-600",
    }
}

fn workflow_status_tone(status: &str) -> &'static str {
    match status {
        "completed" | "success" | "active" => "green",
        "failed" | "error" | "paused" => "red",
        "draft" => "blue",
        _ => "slate",
    }
}

fn prettify_workflow_token(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[component]
fn WorkflowMetricCard(label: String, value: String, tone: String) -> Element {
    let class = match tone.as_str() {
        "blue" => "border-blue-200 bg-blue-50 text-blue-700",
        "green" => "border-emerald-200 bg-emerald-50 text-emerald-700",
        "red" => "border-red-200 bg-red-50 text-red-700",
        _ => "border-slate-200 bg-slate-50 text-slate-700",
    };

    rsx! {
        div { class: format!("rounded-2xl border p-4 {}", class),
            p { class: "text-[10px] font-black uppercase tracking-[0.22em] opacity-70", "{label}" }
            p { class: "text-lg font-black mt-2", "{value}" }
        }
    }
}

#[component]
fn WorkflowRunMetaRow(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-slate-50 px-3 py-2",
            p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "{label}" }
            p { class: "text-xs font-semibold text-slate-700 mt-1 break-all", "{value}" }
        }
    }
}

#[component]
fn WorkflowRunSummaryBlock(label: String, value: String) -> Element {
    rsx! {
        div { class: "rounded-2xl border border-slate-200 bg-slate-50 p-3 space-y-2",
            p { class: "text-[10px] font-black uppercase tracking-[0.22em] text-slate-400", "{label}" }
            pre { class: "text-[11px] text-slate-700 whitespace-pre-wrap break-words font-mono leading-relaxed overflow-x-auto",
                "{value}"
            }
        }
    }
}

#[component]
fn WorkflowStepCard(
    step: WorkflowStep,
    mut steps_resource: Resource<Vec<WorkflowStep>>,
) -> Element {
    let state = use_global_state();
    let db = state.read().db.clone();

    rsx! {
        div { class: "bg-white rounded-3xl border border-slate-200 shadow-sm p-6 space-y-4",
            div { class: "flex flex-col lg:flex-row lg:items-center gap-4",
                div { class: "flex items-center gap-3",
                    input {
                        r#type: "checkbox",
                        checked: step.enabled,
                        onchange: {
                            let db = db.clone();
                            let steps_resource = steps_resource.clone();
                            let step_for_enabled = step.clone();
                            move |_| {
                                let db = db.clone();
                                let mut steps_resource = steps_resource.clone();
                                let mut updated = step_for_enabled.clone();
                                updated.enabled = !updated.enabled;
                                spawn(async move {
                                    let _ = backend::update_workflow_step(&db, updated).await;
                                    steps_resource.restart();
                                });
                            }
                        }
                    }
                    input {
                        class: "w-16 bg-slate-50 border border-slate-200 rounded-xl px-3 py-2 text-sm font-bold text-slate-700",
                        value: "{step.step_order}",
                        oninput: {
                            let db = db.clone();
                            let steps_resource = steps_resource.clone();
                            let step_for_order = step.clone();
                            move |evt| {
                                if let Ok(order) = evt.value().parse::<i64>() {
                                    let db = db.clone();
                                    let mut steps_resource = steps_resource.clone();
                                    let mut updated = step_for_order.clone();
                                    updated.step_order = order.max(1);
                                    spawn(async move {
                                        let _ = backend::update_workflow_step(&db, updated).await;
                                        steps_resource.restart();
                                    });
                                }
                            }
                        }
                    }
                }
                input {
                    class: "flex-1 bg-slate-50 border border-slate-200 rounded-xl px-4 py-3 text-sm font-bold text-slate-800",
                    value: "{step.name}",
                    oninput: {
                        let db = db.clone();
                        let steps_resource = steps_resource.clone();
                        let step_for_name = step.clone();
                        move |evt| {
                            let db = db.clone();
                            let mut steps_resource = steps_resource.clone();
                            let mut updated = step_for_name.clone();
                            updated.name = evt.value();
                            spawn(async move {
                                let _ = backend::update_workflow_step(&db, updated).await;
                                steps_resource.restart();
                            });
                        }
                    }
                }
                button {
                    class: "text-slate-400 hover:text-red-500 transition-colors self-start lg:self-auto",
                    onclick: {
                        let db = db.clone();
                        let steps_resource = steps_resource.clone();
                        move |_| {
                            let db = db.clone();
                            let mut steps_resource = steps_resource.clone();
                            spawn(async move {
                                let _ = backend::delete_workflow_step(&db, step.id).await;
                                steps_resource.restart();
                            });
                        }
                    },
                    MaterialIcon { name: "delete", size: 18 }
                }
            }
            textarea {
                class: "w-full min-h-[150px] bg-slate-50 border border-slate-200 rounded-2xl p-4 text-sm text-slate-700 focus:ring-2 focus:ring-blue-500/10 focus:border-blue-500 transition-all",
                value: "{step.prompt}",
                placeholder: "Describe the instruction this step should run...",
                oninput: {
                    let db = db.clone();
                    let steps_resource = steps_resource.clone();
                    let step_for_prompt = step.clone();
                    move |evt| {
                        let db = db.clone();
                        let mut steps_resource = steps_resource.clone();
                        let mut updated = step_for_prompt.clone();
                        updated.prompt = evt.value();
                        spawn(async move {
                            let _ = backend::update_workflow_step(&db, updated).await;
                            steps_resource.restart();
                        });
                    }
                }
            }
        }
    }
}
