use super::ServerState;
use crate::db::Supervisor;
use crate::engine::utils::discovery::{AgentInfo, ToolInfo};
use axum::{
    extract::{Json, Path, State},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct AddSuperPayload {
    pub name: String,
    pub description: String,
    pub domain: String,
}

#[derive(Deserialize)]
pub struct DeleteSuperPayload {
    pub id: i64,
}

#[derive(Deserialize)]
pub struct SuperLinkPayload {
    pub supervisor_id: i64,
    pub agent_id: i64,
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route(
            "/api/supervisors",
            get(get_supervisors).post(add_supervisor),
        )
        .route("/api/supervisors/update", post(update_supervisor))
        .route("/api/supervisors/delete", post(delete_supervisor))
        .route("/api/supervisors/:id/tools", get(get_supervisor_tools))
        .route("/api/supervisors/:id/agents", get(get_supervisor_agents))
        .route("/api/supervisors/link", post(link_agent_to_supervisor))
        .route(
            "/api/supervisors/unlink",
            post(unlink_agent_from_supervisor),
        )
}

async fn get_supervisors(
    State(state): State<Arc<ServerState>>,
) -> Result<Json<Vec<Supervisor>>, String> {
    crate::backend::get_supervisors(&state.db)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

async fn add_supervisor(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<AddSuperPayload>,
) -> Result<Json<i64>, String> {
    crate::backend::add_supervisor(&state.db, &p.name, &p.description, &p.domain)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

async fn update_supervisor(
    State(state): State<Arc<ServerState>>,
    Json(s): Json<Supervisor>,
) -> Result<(), String> {
    crate::backend::update_supervisor(&state.db, s)
        .await
        .map_err(|e| e.to_string())
}

async fn delete_supervisor(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<DeleteSuperPayload>,
) -> Result<(), String> {
    crate::backend::delete_supervisor(&state.db, p.id)
        .await
        .map_err(|e| e.to_string())
}

async fn get_supervisor_tools(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<ToolInfo>>, String> {
    crate::backend::get_supervisor_tools(&state.db, id)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

async fn get_supervisor_agents(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<AgentInfo>>, String> {
    crate::backend::get_supervisor_agents(&state.db, id)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}

async fn link_agent_to_supervisor(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<SuperLinkPayload>,
) -> Result<(), String> {
    crate::backend::link_agent_to_supervisor(&state.db, p.supervisor_id, p.agent_id)
        .await
        .map_err(|e| e.to_string())
}

async fn unlink_agent_from_supervisor(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<SuperLinkPayload>,
) -> Result<(), String> {
    crate::backend::unlink_agent_from_supervisor(&state.db, p.supervisor_id, p.agent_id)
        .await
        .map_err(|e| e.to_string())
}
