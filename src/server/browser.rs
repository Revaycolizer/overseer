use axum::{
    routing::{get, post, delete},
    extract::{Path, State, Json, ws::WebSocketUpgrade},
    response::IntoResponse,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use super::ServerState;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/browser/sessions", post(create_session))
        .route("/v1/browser/sessions/{id}", get(get_session).delete(close_session))
        .route("/v1/browser/sessions/{id}/navigate", post(navigate))
        .route("/v1/browser/sessions/{id}/screenshot", get(screenshot))
        .route("/v1/browser/sessions/{id}/cdp", get(cdp_relay))
        .route("/v1/browser/memory", get(memory_report))
}

async fn create_session(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<serde_json::Value>, String> {
    let id = state.browser_pool.create_session(None).await.map_err(|e| e.to_string())?;
    Ok(Json(serde_json::json!({"session": {"id": id}})))
}

async fn get_session(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Json<serde_json::Value> {
    Json(serde_json::json!({"exists": state.browser_pool.session_exists(id).await, "id": id}))
}

async fn close_session(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<()>, String> {
    state.browser_pool.close_session(id).await.map_err(|e| e.to_string())?;
    Ok(Json(()))
}

#[derive(Deserialize)]
struct NavigatePayload { url: String }

async fn navigate(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
    Json(payload): Json<NavigatePayload>,
) -> Result<Json<serde_json::Value>, String> {
    let result = state.browser_pool.cdp_call(id, "Page.navigate",
        serde_json::json!({"url": payload.url}),
    ).await.map_err(|e| e.to_string())?;
    Ok(Json(result))
}

async fn screenshot(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, String> {
    let result = state.browser_pool.cdp_call(id, "Page.captureScreenshot",
        serde_json::json!({"format": "png"}),
    ).await.map_err(|e| e.to_string())?;
    Ok(Json(result))
}

async fn cdp_relay(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> impl IntoResponse {
    super::browser_relay::handler(ws, state, id).await
}

async fn memory_report(
    State(state): State<Arc<ServerState>>,
) -> Json<serde_json::Value> {
    let count = state.browser_pool.session_count().await;
    let report = crate::engine::browser::memory_monitor::current_report(count as u64 * 90, count);
    Json(serde_json::json!(report))
}
