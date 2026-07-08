use axum::{
    routing::{get, post},
    extract::{Query, State, Json},
    Router,
};
use std::sync::Arc;
use serde::Deserialize;
use super::ServerState;
use crate::engine::utils::discovery::AgentInfo;

#[derive(Deserialize)]
pub struct AddAgentPayload {
    pub name: String,
    pub description: String,
    pub domain: String,
}

#[derive(Deserialize)]
pub struct DomainQuery {
    pub domain: String,
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/agents", get(get_agents).post(add_agent))
        .route("/api/agents/all", get(get_all_agents))
        .route("/api/agents/update", post(update_agent))
        .route("/api/agents/by_domain", get(get_agent_by_domain))
}

async fn get_agents(State(state): State<Arc<ServerState>>) -> Result<Json<Vec<AgentInfo>>, String> {
    crate::backend::get_agents(&state.db).await.map(Json).map_err(|e| e.to_string())
}

async fn get_all_agents(State(state): State<Arc<ServerState>>) -> Result<Json<Vec<AgentInfo>>, String> {
    crate::backend::get_all_agents(&state.db).await.map(Json).map_err(|e| e.to_string())
}

async fn add_agent(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<AddAgentPayload>,
) -> Result<Json<i64>, String> {
    crate::backend::add_agent(&state.db, &p.name, &p.description, &p.domain).await.map(Json).map_err(|e| e.to_string())
}

async fn update_agent(
    State(state): State<Arc<ServerState>>,
    Json(a): Json<AgentInfo>,
) -> Result<(), String> {
    crate::backend::update_agent(&state.db, a).await.map_err(|e| e.to_string())
}

async fn get_agent_by_domain(
    State(state): State<Arc<ServerState>>,
    Query(q): Query<DomainQuery>,
) -> Result<Json<Option<AgentInfo>>, String> {
    crate::backend::get_agent_by_domain(&state.db, &q.domain).await.map(Json).map_err(|e| e.to_string())
}
