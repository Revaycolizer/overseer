use crate::engine::orchestrator::core::event_logger::record_event;
use crate::engine::orchestrator::native::native_checks::{
    infer_native_computer_domain, is_native_computer_fast_path_request,
};
use crate::engine::orchestrator::native::native_render::render_native_os_tool_response;
use crate::engine::utils::discovery::ToolInfo;

pub async fn try_native_computer_shortcut(
    db: &crate::db::Database,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    query: &str,
) -> Option<String> {
    if infer_native_computer_domain(query).is_none()
        && !is_native_computer_fast_path_request(query, "computer")
    {
        return None;
    }

    let tool = crate::backend::get_tools(db)
        .await
        .ok()
        .and_then(|tools| tools.into_iter().find(|tool| tool.name == "os_tool"))
        .unwrap_or_else(|| ToolInfo {
            id: 0,
            name: "os_tool".to_string(),
            description: "Native system access".to_string(),
            code: "// Native Implementation".to_string(),
            language: "rust".to_string(),
            script_path: None,
            required_intent: None,
            safety_level: "standard".to_string(),
            version: "1.0.0".to_string(),
        });

    let input_json = serde_json::json!({ "query": query });
    record_event(
        db,
        run_id,
        event_sequence,
        "native_computer_shortcut_started",
        "running",
        Some("computer"),
        Some(query),
        Some("os_tool"),
        None,
        Some("Handling native computer request."),
        Some(&input_json.to_string()),
    )
    .await;

    // Route the native fast-path through the kernel too — this is a real
    // computer action and must not bypass policy/approval enforcement.
    let raw = match crate::engine::kernel::kernel_execute(
        db,
        run_id,
        "computer",
        query,
        &tool,
        input_json,
        &[tool.name.clone()],
    )
    .await
    {
        overseer_kernel::ExecutionOutcome::Executed { output } => output,
        other => {
            let msg = other.agent_response();
            record_event(
                db,
                run_id,
                event_sequence,
                "native_computer_shortcut_blocked",
                "failed",
                Some("computer"),
                Some(query),
                Some("os_tool"),
                None,
                Some("Native computer shortcut blocked by kernel policy."),
                Some(&msg),
            )
            .await;
            return Some(msg);
        }
    };

    let rendered = render_native_os_tool_response(&raw);
    record_event(
        db,
        run_id,
        event_sequence,
        "native_computer_shortcut_completed",
        "completed",
        Some("computer"),
        Some(query),
        Some("os_tool"),
        None,
        Some("Completed native computer shortcut."),
        Some(&raw),
    )
    .await;

    if let Some(run_id) = run_id {
        let _ =
            crate::backend::finish_execution_run(db, run_id, "completed", Some(&rendered), None)
                .await;
    }
    Some(rendered)
}
