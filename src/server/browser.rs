use axum::{
    routing::{get, post, delete},
    extract::{Path, State, Json},
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
        .route("/v1/browser/memory", get(memory_report))
}

#[derive(Serialize)]
struct SessionInfo {
    id: i64,
    cdp_ws_url: String,
}

#[derive(Serialize)]
struct CreateSessionResponse {
    session: SessionInfo,
}

async fn create_session(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<CreateSessionResponse>, String> {
    let id = state.browser_pool.create_session(None).await.map_err(|e| e.to_string())?;
    Ok(Json(CreateSessionResponse {
        session: SessionInfo { id, cdp_ws_url: String::new() },
    }))
}

#[derive(Serialize)]
struct GetSessionResponse {
    exists: bool,
    id: i64,
}

async fn get_session(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Json<GetSessionResponse> {
    Json(GetSessionResponse { exists: state.browser_pool.session_exists(id).await, id })
}

async fn close_session(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<()>, String> {
    state.browser_pool.close_session(id).await.map_err(|e| e.to_string())?;
    Ok(Json(()))
}

#[derive(Deserialize)]
struct NavigatePayload {
    url: String,
}

async fn navigate(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
    Json(payload): Json<NavigatePayload>,
) -> Result<Json<serde_json::Value>, String> {
    let ws_url = format!("ws://127.0.0.1:{}/devtools/browser", 9300 + id as u16);
    let result = crate::engine::browser::cdp::navigate_and_wait(
        &ws_url, &payload.url,
    ).await.map_err(|e| e.to_string())?;
    Ok(Json(result))
}

#[derive(Serialize)]
struct MemoryReport {
    total_ram_mb: u64,
    used_ram_mb: u64,
    available_ram_mb: u64,
    browser_pool_mb: u64,
}

async fn memory_report() -> Json<MemoryReport> {
    let mut sys = sysinfo::System::new();
    sys.refresh_memory();
    let total = sys.total_memory() / 1024 / 1024;
    let used = sys.used_memory() / 1024 / 1024;
    Json(MemoryReport {
        total_ram_mb: total,
        used_ram_mb: used,
        available_ram_mb: total.saturating_sub(used),
        browser_pool_mb: 0,
    })
}

async fn screenshot(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, String> {
    let ws_url = format!("ws://127.0.0.1:{}/devtools/browser", 9300 + id as u16);
    let result = crate::engine::browser::cdp::cdp_call(
        &ws_url, "Page.captureScreenshot",
        serde_json::json!({"format": "png"}),
    ).await.map_err(|e| e.to_string())?;
    Ok(Json(result))
}
