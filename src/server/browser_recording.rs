use axum::{
    routing::{get, post},
    extract::{Path, State, Json, ws::{WebSocket, WebSocketUpgrade, Message as WsMessage}},
    response::IntoResponse,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use super::ServerState;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/browser/sessions/{id}/recording/start", post(start_recording))
        .route("/v1/browser/sessions/{id}/recording/stop", post(stop_recording))
        .route("/v1/browser/sessions/{id}/live", get(live_view))
}

#[derive(Deserialize)]
struct StartPayload { quality: Option<u8> }

async fn start_recording(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
    Json(payload): Json<StartPayload>,
) -> Result<Json<serde_json::Value>, String> {
    let rx = state.browser_pool.start_recording(id, payload.quality.unwrap_or(75))
        .await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"recording": true})))
}

async fn stop_recording(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, String> {
    state.browser_pool.stop_recording(id).await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"recording": false})))
}

async fn live_view(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    ws.on_upgrade(move |client_ws| stream_live(client_ws, state, id))
}

async fn stream_live(mut client_ws: WebSocket, state: Arc<ServerState>, id: i64) {
    let mut rx = match state.browser_pool.start_recording(id, 75).await {
        Ok(r) => r,
        Err(_) => return,
    };
    let (mut tx, _) = client_ws.split();

    while let Ok(frame) = rx.recv().await {
        if tx.send(WsMessage::Binary(frame.into())).await.is_err() { break; }
    }
}
