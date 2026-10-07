use super::error::DynamicToolError;

fn pool() -> Result<std::sync::Arc<crate::engine::browser::pool::BrowserPool>, DynamicToolError> {
    crate::engine::browser::global::get()
        .cloned()
        .ok_or(DynamicToolError::ExecutionFailed {
            tool: "browser".to_string(),
            details: "Browser pool not initialized".to_string(),
        })
}

pub async fn navigate(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let sid = args
        .get("session_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| DynamicToolError::ExecutionFailed {
            tool: "browser_navigate".to_string(),
            details: "missing session_id".to_string(),
        })?;
    let url = args.get("url").and_then(|v| v.as_str()).ok_or_else(|| {
        DynamicToolError::ExecutionFailed {
            tool: "browser_navigate".to_string(),
            details: "missing url".to_string(),
        }
    })?;
    pool()?
        .cdp_call(sid, "Page.navigate", serde_json::json!({"url": url}))
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: "browser_navigate".to_string(),
            details: e.to_string(),
        })?;
    Ok(format!("Navigated to {}", url))
}

/// Read the current page's visible text (CDP Runtime.evaluate on innerText).
pub async fn read_page(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let sid = args
        .get("session_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| DynamicToolError::ExecutionFailed {
            tool: "browser_read".to_string(),
            details: "missing session_id".to_string(),
        })?;
    let v = pool()?
        .cdp_call(
            sid,
            "Runtime.evaluate",
            serde_json::json!({
                "expression": "document.body ? document.body.innerText : ''",
                "returnByValue": true
            }),
        )
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: "browser_read".to_string(),
            details: e.to_string(),
        })?;
    let text = v["result"]["value"].as_str().unwrap_or("").to_string();
    let trimmed: String = text.chars().take(8000).collect();
    Ok(serde_json::json!({
        "summary": format!("read {} chars from page", text.len()),
        "text": trimmed,
    })
    .to_string())
}

/// Click the first link/button whose visible text contains `text` (case-insensitive).
pub async fn click_text(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let sid = args
        .get("session_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| DynamicToolError::ExecutionFailed {
            tool: "browser_click".to_string(),
            details: "missing session_id".to_string(),
        })?;
    let text = args
        .get("text")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    // Embed the target text as a safe JS string literal.
    let needle = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string());
    let expression = format!(
        "(function(t){{\
           t=t.toLowerCase();\
           const els=[...document.querySelectorAll('a,button,[role=\\\"button\\\"],input[type=\\\"submit\\\"]')];\
           const el=els.find(e=>((e.innerText||e.value||'').trim().toLowerCase()).includes(t));\
           if(el){{el.click();return 'clicked: '+((el.innerText||el.value||'').trim().slice(0,80));}}\
           return 'no element matched';\
         }})({})",
        needle
    );
    let v = pool()?
        .cdp_call(
            sid,
            "Runtime.evaluate",
            serde_json::json!({ "expression": expression, "returnByValue": true }),
        )
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: "browser_click".to_string(),
            details: e.to_string(),
        })?;
    let result = v["result"]["value"]
        .as_str()
        .unwrap_or("(no result)")
        .to_string();
    Ok(serde_json::json!({ "summary": result }).to_string())
}

pub async fn screenshot(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let sid = args
        .get("session_id")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| DynamicToolError::ExecutionFailed {
            tool: "browser_screenshot".to_string(),
            details: "missing session_id".to_string(),
        })?;
    let result = pool()?
        .cdp_call(
            sid,
            "Page.captureScreenshot",
            serde_json::json!({"format": "png"}),
        )
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: "browser_screenshot".to_string(),
            details: e.to_string(),
        })?;
    Ok(result.to_string())
}
