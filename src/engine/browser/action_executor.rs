use std::sync::Arc;
use super::action_types::*;
use super::cdp::CdpConnection;
use super::dom_snapshot::DomSnapshot;
use super::wait_strategies;

pub struct ActionExecutor {
    pub cdp: Arc<CdpConnection>,
    pub policy: ActionPolicy,
}

impl ActionExecutor {
    pub fn new(cdp: Arc<CdpConnection>, policy: ActionPolicy) -> Self {
        Self { cdp, policy }
    }

    pub async fn navigate(&self, url: &str) -> anyhow::Result<ActionResult> {
        let start = std::time::Instant::now();
        let before = DomSnapshot::capture(&self.cdp).await?;

        self.cdp.call("Page.navigate", serde_json::json!({"url": url})).await?;
        let _ = wait_strategies::wait_for_navigation(&self.cdp, self.policy.action_timeout_ms).await;

        let after = DomSnapshot::capture(&self.cdp).await?;
        let nav = before.url != after.url;
        Ok(ActionResult {
            success: nav,
            url: after.url.clone(),
            title: after.title.clone(),
            dom_changed: before != after,
            navigation_occurred: nav,
            retries_used: 0,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }

    pub async fn click(&self, selector: &str) -> anyhow::Result<ActionResult> {
        let start = std::time::Instant::now();
        let before = DomSnapshot::capture(&self.cdp).await?;

        wait_strategies::wait_for_selector(&self.cdp, selector, self.policy.action_timeout_ms).await?;
        let sel = serde_json::json!(selector).to_string();
        self.cdp.call("Runtime.evaluate", serde_json::json!({
            "expression": format!("document.querySelector({}).click()", sel),
        })).await?;

        let after = DomSnapshot::capture(&self.cdp).await?;
        let nav = before.url != after.url;
        Ok(ActionResult {
            success: true,
            url: after.url.clone(),
            title: after.title.clone(),
            dom_changed: before != after,
            navigation_occurred: nav,
            retries_used: 0,
            duration_ms: start.elapsed().as_millis() as u64,
        })
    }
}
