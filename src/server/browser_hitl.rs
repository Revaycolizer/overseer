use super::ServerState;
use axum::{
    extract::{Json, Path, State},
    routing::post,
    Router,
};
use std::sync::Arc;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/v1/browser/sessions/:id/resume", post(resume_session))
}

async fn resume_session(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<()>, String> {
    state
        .browser_pool
        .hitl_resume(id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(Json(()))
}
