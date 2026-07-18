use axum::{
    extract::{Path, State, ws::{WebSocket, WebSocketUpgrade, Message as WsMessage}},
    response::IntoResponse,
};
use futures::{SinkExt, StreamExt};
use std::sync::Arc;
use super::ServerState;

pub async fn handler(
    ws: WebSocketUpgrade,
    state: Arc<ServerState>,
    id: i64,
) -> impl IntoResponse {
    ws.on_upgrade(move |client_ws| relay(client_ws, state, id))
}

async fn relay(client_ws: WebSocket, state: Arc<ServerState>, id: i64) {
    let Some(obscura_url) = state.browser_pool.get_cdp_url(id).await else { return };
    let obscura_ws = match tokio_tungstenite::connect_async(&obscura_url).await {
        Ok((ws, _)) => ws,
        Err(_) => return,
    };

    let (mut client_tx, mut client_rx) = client_ws.split();
    let (mut obscura_tx, mut obscura_rx) = obscura_ws.split();

    let c2o = async {
        while let Some(Ok(msg)) = client_rx.next().await {
            let data = match msg {
                WsMessage::Text(t) => t.to_string(),
                WsMessage::Binary(d) => String::from_utf8_lossy(&d).to_string(),
                WsMessage::Close(_) => break,
                _ => continue,
            };
            let _ = obscura_tx.send(tokio_tungstenite::tungstenite::Message::Text(data)).await;
        }
    };

    let o2c = async {
        while let Some(Ok(msg)) = obscura_rx.next().await {
            let payload = match msg {
                tokio_tungstenite::tungstenite::Message::Text(t) => WsMessage::Text(t),
                tokio_tungstenite::tungstenite::Message::Binary(d) => WsMessage::Binary(d),
                tokio_tungstenite::tungstenite::Message::Close(_) => break,
                _ => continue,
            };
            if client_tx.send(payload).await.is_err() { break; }
        }
    };

    tokio::select! {
        _ = c2o => {}
        _ = o2c => {}
    }
}
