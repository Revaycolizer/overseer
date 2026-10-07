use axum::{
    routing::{get, post},
    extract::{Path, State, Json, ws::{WebSocket, WebSocketUpgrade, Message as WsMessage}},
    response::IntoResponse,
    body::Bytes,
    http::StatusCode,
    Router,
};
use futures::{SinkExt, StreamExt};
use serde::Deserialize;
use std::sync::Arc;
use super::ServerState;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/browser/sessions/:id/recording/start", post(start_recording))
        .route("/v1/browser/sessions/:id/recording/stop", post(stop_recording))
        .route("/v1/browser/sessions/:id/recording", get(get_recording))
        .route("/v1/browser/sessions/:id/recording/events", get(get_events))
        .route("/v1/browser/sessions/:id/live", get(live_view))
}

#[derive(Deserialize)]
struct StartPayload { quality: Option<u8> }

async fn start_recording(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
    Json(payload): Json<StartPayload>,
) -> Result<Json<serde_json::Value>, String> {
    let _rx = state.browser_pool.start_recording(id, payload.quality.unwrap_or(75))
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

async fn get_recording(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<(StatusCode, Bytes), (StatusCode, String)> {
    let path = state.browser_pool.get_profile_path(id).await
        .ok_or((StatusCode::NOT_FOUND, "Session not found".into()))?;
    let video = path.join("recording").join("recording.mp4");
    let avi = path.join("recording").join("recording.avi");
    let file = if video.exists() { video } else if avi.exists() { avi } else {
        return Err((StatusCode::NOT_FOUND, "No recording found".into()));
    };
    let bytes = tokio::fs::read(&file).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    Ok((StatusCode::OK, Bytes::from(bytes)))
}

async fn get_events(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, String> {
    let path = state.browser_pool.get_profile_path(id).await
        .ok_or("Session not found".to_string())?;
    let events_path = path.join("recording").join("event_log.jsonl");
    let content = tokio::fs::read_to_string(&events_path).await.unwrap_or_default();
    let events: Vec<serde_json::Value> = content.lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();
    Ok(Json(serde_json::json!(events)))
}

async fn live_view(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    ws.on_upgrade(move |client_ws| stream_live(client_ws, state, id))
}

async fn stream_live(client_ws: WebSocket, state: Arc<ServerState>, id: i64) {
    let mut rx = match state.browser_pool.start_recording(id, 75).await {
        Ok(r) => r,
        Err(_) => return,
    };
    let (mut tx, _) = client_ws.split();

    while let Ok(frame) = rx.recv().await {
        if tx.send(WsMessage::Binary(frame.into())).await.is_err() { break; }
    }
}
