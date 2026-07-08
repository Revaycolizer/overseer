use axum::{
    routing::{get, post},
    extract::{State, Json},
    Router,
};
use std::sync::Arc;
use serde::Deserialize;
use super::ServerState;
use crate::engine::utils::discovery::ToolInfo;

#[derive(Deserialize)]
pub struct AddToolPayload {
    pub name: String,
    pub description: String,
    pub language: String,
}

#[derive(Deserialize)]
pub struct ToolLinkPayload {
    pub agent_id: i64,
    pub tool_id: i64,
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/tools", get(get_tools).post(add_tool))
        .route("/api/tools/update", post(update_tool))
        .route("/api/tools/link", post(add_tool_to_agent))
        .route("/api/tools/unlink", post(remove_tool_from_agent))
}

async fn get_tools(State(state): State<Arc<ServerState>>) -> Result<Json<Vec<ToolInfo>>, String> {
    crate::backend::get_tools(&state.db).await.map(Json).map_err(|e| e.to_string())
}

async fn add_tool(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<AddToolPayload>,
) -> Result<Json<i64>, String> {
    crate::backend::add_tool(&state.db, &p.name, &p.description, &p.language).await.map(Json).map_err(|e| e.to_string())
}

async fn update_tool(
    State(state): State<Arc<ServerState>>,
    Json(t): Json<ToolInfo>,
) -> Result<(), String> {
    crate::backend::update_tool(&state.db, t).await.map_err(|e| e.to_string())
}

async fn add_tool_to_agent(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<ToolLinkPayload>,
) -> Result<(), String> {
    crate::backend::add_tool_to_agent(&state.db, p.agent_id, p.tool_id).await.map_err(|e| e.to_string())
}

async fn remove_tool_from_agent(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<ToolLinkPayload>,
) -> Result<(), String> {
    crate::backend::remove_tool_from_agent(&state.db, p.agent_id, p.tool_id).await.map_err(|e| e.to_string())
}
