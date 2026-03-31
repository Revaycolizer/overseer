use crate::engine::utils::discovery::ToolInfo;
use crate::engine::orchestrator::record_event;
use crate::engine::orchestrator::native_render::render_native_os_tool_response;
use crate::engine::orchestrator::native_checks::{infer_native_computer_domain, is_native_computer_fast_path_request};

pub async fn try_native_computer_shortcut(
    db: &crate::db::Database,
    run_id: Option<i64>,
    event_sequence: &mut i64,
    query: &str,
) -> Option<String> {
    if infer_native_computer_domain(query).is_none() && !is_native_computer_fast_path_request(query, "computer") {
        return None;
    }

    let tool = crate::backend::get_tools(db).await.ok()
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
    record_event(db, run_id, event_sequence, "native_computer_shortcut_started", "running", Some("computer"), Some(query), Some("os_tool"), None, Some("Handling native computer request."), Some(&input_json.to_string())).await;

    let raw = match crate::engine::tools::call_dynamic_tool(&tool, input_json).await {
        Ok(raw) => raw,
        Err(e) => {
            let error_msg = format!("Native shortcut failed: {}", e);
            record_event(db, run_id, event_sequence, "native_computer_shortcut_failed", "failed", Some("computer"), Some(query), Some("os_tool"), None, Some(&error_msg), None).await;
            return Some(error_msg);
        }
    };

    let rendered = render_native_os_tool_response(&raw);
    record_event(db, run_id, event_sequence, "native_computer_shortcut_completed", "completed", Some("computer"), Some(query), Some("os_tool"), None, Some("Completed native computer shortcut."), Some(&raw)).await;

    if let Some(run_id) = run_id {
        let _ = crate::backend::finish_execution_run(db, run_id, "completed", Some(&rendered), None).await;
    }
    Some(rendered)
}
