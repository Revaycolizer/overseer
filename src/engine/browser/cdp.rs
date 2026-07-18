use anyhow::Result;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

pub struct CdpConnection {
    ws: Arc<Mutex<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>>>,
}

impl CdpConnection {
    pub async fn connect(ws_url: &str) -> Result<Self> {
        let (ws, _) = connect_async(ws_url).await?;
        Ok(Self { ws: Arc::new(Mutex::new(ws)) })
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id: u64 = rand::random();
        let cmd = serde_json::json!({"id": id, "method": method, "params": params});
        let mut ws = self.ws.lock().await;
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
                Some(Ok(Message::Ping(_))) => {
                    let _ = ws.send(Message::Pong(vec![])).await;
                }
                Some(Ok(Message::Binary(_))) | Some(Ok(Message::Pong(_))) => continue,
                Some(Err(e)) => anyhow::bail!("WS error: {e}"),
                None => anyhow::bail!("WS closed"),
                _ => continue,
            }
        }
    }

    pub fn start_heartbeat(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(15));
            loop {
                interval.tick().await;
                let mut ws = self.ws.lock().await;
                if ws.send(Message::Ping(vec![])).await.is_err() {
                    break;
                }
            }
        });
    }
}

pub async fn cdp_call(ws_url: &str, method: &str, params: Value) -> Result<Value> {
    let conn = CdpConnection::connect(ws_url).await?;
    conn.call(method, params).await
}

pub async fn navigate_and_wait(ws_url: &str, url: &str) -> Result<Value> {
    let conn = CdpConnection::connect(ws_url).await?;
    conn.call("Page.enable", serde_json::json!({})).await?;
    let result = conn.call("Page.navigate", serde_json::json!({"url": url})).await;
    let _ = conn.call("Page.domContentEventFired", serde_json::json!({})).await;
    result
}

pub async fn block_resources(ws_url: &str) -> Result<()> {
    let conn = CdpConnection::connect(ws_url).await?;
    conn.call("Network.enable", serde_json::json!({})).await?;
    conn.call("Network.setBlockedURLs", serde_json::json!({
        "urls": [
            "*.png", "*.jpg", "*.gif", "*.svg", "*.ico",
            "*.woff", "*.woff2", "*.ttf", "*.eot",
            "*.mp4", "*.mp3", "*.webm",
            "*/analytics/*", "*/tracking/*", "*/ads/*", "*/beacon/*",
        ]
    })).await?;
    Ok(())
}
