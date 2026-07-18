use super::error::DynamicToolError;

fn pool() -> Result<std::sync::Arc<crate::engine::browser::pool::BrowserPool>, DynamicToolError> {
    crate::engine::browser::global::get().cloned().ok_or(DynamicToolError::ExecutionFailed {
        tool: "browser".to_string(), details: "Browser pool not initialized".to_string(),
    })
}

pub async fn navigate(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let sid = args.get("session_id").and_then(|v| v.as_i64()).ok_or_else(|| DynamicToolError::ExecutionFailed {
        tool: "browser_navigate".to_string(), details: "missing session_id".to_string(),
    })?;
    let url = args.get("url").and_then(|v| v.as_str()).ok_or_else(|| DynamicToolError::ExecutionFailed {
        tool: "browser_navigate".to_string(), details: "missing url".to_string(),
    })?;
    pool()?.cdp_call(sid, "Page.navigate", serde_json::json!({"url": url})).await
        .map_err(|e| DynamicToolError::ExecutionFailed { tool: "browser_navigate".to_string(), details: e.to_string() })?;
    Ok(format!("Navigated to {}", url))
}

pub async fn screenshot(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let sid = args.get("session_id").and_then(|v| v.as_i64()).ok_or_else(|| DynamicToolError::ExecutionFailed {
        tool: "browser_screenshot".to_string(), details: "missing session_id".to_string(),
    })?;
    let result = pool()?.cdp_call(sid, "Page.captureScreenshot", serde_json::json!({"format": "png"})).await
        .map_err(|e| DynamicToolError::ExecutionFailed { tool: "browser_screenshot".to_string(), details: e.to_string() })?;
    Ok(result.to_string())
}
