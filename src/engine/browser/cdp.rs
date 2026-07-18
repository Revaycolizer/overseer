use anyhow::Result;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

pub async fn cdp_call(ws_url: &str, method: &str, params: Value) -> Result<Value> {
    let (mut ws, _) = connect_async(ws_url).await?;
    let id: u64 = rand::random();
    let cmd = serde_json::json!({
        "id": id,
        "method": method,
        "params": params,
    });
    ws.send(Message::Text(cmd.to_string())).await?;

    loop {
        match ws.next().await {
            Some(Ok(Message::Text(text))) => {
                let msg: Value = serde_json::from_str(&text)?;
                if msg.get("id") == Some(&serde_json::json!(id)) {
                    if let Some(result) = msg.get("result") {
                        return Ok(result.clone());
                    }
                    if let Some(err) = msg.get("error") {
                        anyhow::bail!("CDP error: {}", err);
                    }
                }
            }
            Some(Ok(Message::Binary(_))) => continue,
            Some(Err(e)) => anyhow::bail!("WS error: {e}"),
            None => anyhow::bail!("WS closed without response"),
            _ => continue,
        }
    }
}

pub async fn block_resources(ws_url: &str) -> Result<()> {
    cdp_call(ws_url, "Network.enable", serde_json::json!({})).await?;
    cdp_call(ws_url, "Network.setBlockedURLs", serde_json::json!({
        "urls": [
            "*.png", "*.jpg", "*.gif", "*.svg", "*.ico",
            "*.woff", "*.woff2", "*.ttf", "*.eot",
            "*.mp4", "*.mp3", "*.webm",
            "*/analytics/*", "*/tracking/*", "*/ads/*", "*/beacon/*",
        ]
    })).await?;
    Ok(())
}
