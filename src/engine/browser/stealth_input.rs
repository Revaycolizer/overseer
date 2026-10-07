use crate::engine::browser::cdp::CdpConnection;
use std::sync::Arc;

pub async fn human_like_click(cdp: &Arc<CdpConnection>, x: f64, y: f64) -> anyhow::Result<()> {
    let steps = 8 + rand::random::<u8>() % 5;
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        cdp.call(
            "Input.dispatchMouseEvent",
            serde_json::json!({
                "type": "mouseMoved",
                "x": x * t, "y": y * t,
            }),
        )
        .await?;
        tokio::time::sleep(std::time::Duration::from_millis(
            10 + rand::random::<u64>() % 20,
        ))
        .await;
    }
    cdp.call(
        "Input.dispatchMouseEvent",
        serde_json::json!({
            "type": "mousePressed", "button": "left",
            "x": x, "y": y, "clickCount": 1,
        }),
    )
    .await?;
    cdp.call(
        "Input.dispatchMouseEvent",
        serde_json::json!({
            "type": "mouseReleased", "button": "left",
            "x": x, "y": y, "clickCount": 1,
        }),
    )
    .await?;
    Ok(())
}

pub async fn human_like_type(cdp: &Arc<CdpConnection>, text: &str) -> anyhow::Result<()> {
    for ch in text.chars() {
        let key = match ch {
            '\n' => "Enter",
            '\t' => "Tab",
            c if c.is_ascii_control() => continue,
            c => &c.to_string(),
        };
        cdp.call(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": "keyDown", "key": key,
            }),
        )
        .await?;
        cdp.call(
            "Input.dispatchKeyEvent",
            serde_json::json!({
                "type": "keyUp", "key": key,
            }),
        )
        .await?;
        if !ch.is_ascii_control() {
            tokio::time::sleep(std::time::Duration::from_millis(
                50 + rand::random::<u64>() % 100,
            ))
            .await;
        }
    }
    Ok(())
}
