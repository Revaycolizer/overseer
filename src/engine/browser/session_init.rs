use super::cdp::CdpConnection;
use super::session::BrowserSession;
use std::sync::Arc;

pub async fn init_session(session: &mut BrowserSession) {
    let cdp = match CdpConnection::connect(&session.cdp_ws_url).await {
        Ok(c) => Arc::new(c),
        Err(e) => {
            tracing::warn!(?e, "Failed to connect CDP for session {}", session.id);
            return;
        }
    };
    cdp.clone().start_heartbeat();

    let bg_cdp = cdp.clone();
    let sid = session.id;
    tokio::spawn(async move {
        let _ = bg_cdp.call("Network.enable", serde_json::json!({})).await;
        let _ = bg_cdp
            .call(
                "Network.setBlockedURLs",
                serde_json::json!({
                    "urls": [
                        "*.png", "*.jpg", "*.gif", "*.svg", "*.ico",
                        "*.woff", "*.woff2", "*.ttf", "*.eot",
                        "*.mp4", "*.mp3", "*.webm",
                        "*/analytics/*", "*/tracking/*", "*/ads/*", "*/beacon/*",
                    ]
                }),
            )
            .await;
        let width = 1280u32 + (sid as u8 as u32 % 86);
        let height = 800u32 + ((sid >> 8) as u8 as u32 % 68);
        let _ = bg_cdp
            .call(
                "Emulation.setDeviceMetricsOverride",
                serde_json::json!({
                    "width": width, "height": height,
                    "deviceScaleFactor": 1, "mobile": false,
                }),
            )
            .await;
    });

    session.cdp = Some(cdp);
}
