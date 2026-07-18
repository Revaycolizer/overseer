use crate::engine::browser::cdp::CdpConnection;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomSnapshot {
    pub url: String,
    pub title: String,
    pub body_len: usize,
}

impl DomSnapshot {
    pub async fn capture(cdp: &Arc<CdpConnection>) -> anyhow::Result<Self> {
        let url = cdp.call("Runtime.evaluate", serde_json::json!({
            "expression": "location.href",
        })).await?;
        let title = cdp.call("Runtime.evaluate", serde_json::json!({
            "expression": "document.title",
        })).await?;
        let body = cdp.call("Runtime.evaluate", serde_json::json!({
            "expression": "(document.body?.innerText || '').length",
        })).await?;

        Ok(Self {
            url: url.pointer("/result/value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            title: title.pointer("/result/value").and_then(|v| v.as_str()).unwrap_or("").to_string(),
            body_len: body.pointer("/result/value").and_then(|v| v.as_u64()).unwrap_or(0) as usize,
        })
    }
}
