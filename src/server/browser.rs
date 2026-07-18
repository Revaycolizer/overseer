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
