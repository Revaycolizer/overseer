use crate::engine::browser::cdp::CdpConnection;
use std::time::Duration;
use std::sync::Arc;

pub async fn wait_for_selector(cdp: &Arc<CdpConnection>, selector: &str, timeout_ms: u64) -> anyhow::Result<bool> {
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
    while std::time::Instant::now() < deadline {
        let result = cdp.call("Runtime.evaluate", serde_json::json!({
            "expression": format!("!!document.querySelector('{}')", selector),
        })).await?;
        if result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_bool()).unwrap_or(false) {
            return Ok(true);
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    anyhow::bail!("Selector '{}' not found within {}ms", selector, timeout_ms)
}

pub async fn wait_for_navigation(cdp: &Arc<CdpConnection>, timeout_ms: u64) -> anyhow::Result<()> {
    let _ = cdp.call("Page.enable", serde_json::json!({})).await?;
    let deadline = std::time::Instant::now() + Duration::from_millis(timeout_ms);
    while std::time::Instant::now() < deadline {
        let result = cdp.call("Runtime.evaluate", serde_json::json!({
            "expression": "document.readyState",
        })).await?;
        let state = result.get("result").and_then(|r| r.get("value")).and_then(|v| v.as_str()).unwrap_or("");
        if state == "complete" { return Ok(()); }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    anyhow::bail!("Page did not finish loading within {}ms", timeout_ms)
}
