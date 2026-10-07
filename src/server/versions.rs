use super::ServerState;
use crate::backend::*;
use axum::{
    extract::{Json, Query, State},
    routing::{get, post},
    Router,
};
use serde::Deserialize;
use std::sync::Arc;

#[derive(Deserialize)]
pub struct ContentQuery {
    pub entity_type: String,
    pub entity_id: i64,
    pub tag: String,
}

#[derive(Deserialize)]
pub struct HistoryQuery {
    pub entity_type: String,
    pub entity_id: i64,
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/version/publish", post(publish))
        .route("/api/version/publish_agent", post(publish_agent))
        .route("/api/version/publish_tool", post(publish_tool))
        .route("/api/version/rollback", post(rollback))
        .route("/api/version/content", get(get_content))
        .route("/api/version/history", get(get_history))
}

async fn publish(
    State(s): State<Arc<ServerState>>,
    Json(p): Json<serde_json::Value>,
) -> Result<(), String> {
    let t = p["entity_type"].as_str().unwrap_or("");
    let id = p["entity_id"].as_i64().unwrap_or(0);
    let tag = p["tag"].as_str().unwrap_or("");
    let content = p["content"].as_str().unwrap_or("");
    let d = p["description"].as_str().unwrap_or("");
    crate::backend::publish_version(&s.db, t, id, tag, content, d)
        .await
        .map_err(|e| e.to_string())
}

async fn publish_agent(
    State(s): State<Arc<ServerState>>,
    Json(p): Json<serde_json::Value>,
) -> Result<(), String> {
    let id = p["agent_id"].as_i64().unwrap_or(0);
    let tag = p["tag"].as_str().unwrap_or("");
    let d = p["description"].as_str().unwrap_or("");
    crate::backend::publish_agent_version(&s.db, id, tag, d)
        .await
        .map_err(|e| e.to_string())
}

async fn publish_tool(
    State(s): State<Arc<ServerState>>,
    Json(p): Json<serde_json::Value>,
) -> Result<(), String> {
    let id = p["tool_id"].as_i64().unwrap_or(0);
    let tag = p["tag"].as_str().unwrap_or("");
    let d = p["description"].as_str().unwrap_or("");
    crate::backend::publish_tool_version(&s.db, id, tag, d)
        .await
        .map_err(|e| e.to_string())
}

async fn rollback(
    State(s): State<Arc<ServerState>>,
    Json(p): Json<serde_json::Value>,
) -> Result<(), String> {
    let t = p["entity_type"].as_str().unwrap_or("");
    let id = p["entity_id"].as_i64().unwrap_or(0);
    let tag = p["tag"].as_str().unwrap_or("");
    crate::backend::rollback_version(&s.db, t, id, tag)
        .await
        .map_err(|e| e.to_string())
}

async fn get_content(
    State(s): State<Arc<ServerState>>,
    Query(q): Query<ContentQuery>,
) -> Result<Json<String>, String> {
    crate::backend::get_version_content(&s.db, &q.entity_type, q.entity_id, &q.tag)
        .await
        .map(|opt| Json(opt.unwrap_or_default()))
        .map_err(|e| e.to_string())
}

async fn get_history(
    State(s): State<Arc<ServerState>>,
    Query(q): Query<HistoryQuery>,
) -> Result<Json<Vec<EntityVersion>>, String> {
    crate::backend::get_version_history(&s.db, &q.entity_type, q.entity_id)
        .await
        .map(Json)
        .map_err(|e| e.to_string())
}
