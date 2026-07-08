use axum::{
    routing::{get, post},
    extract::{Query, State, Json},
    Router,
};
use std::sync::Arc;
use serde::Deserialize;
use super::ServerState;
use crate::backend::*;

#[derive(Deserialize)]
pub struct KeyQuery { pub key: String }
#[derive(Deserialize)]
pub struct KeyVal { pub key: String, pub value: String }
#[derive(Deserialize)]
pub struct OrchestratePayload { pub conversation_id: i64, pub query: String }

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/settings", get(get_setting).post(set_setting))
        .route("/api/settings/all", get(get_all_settings))
        .route("/api/supervision/state", get(get_supervision_state))
        .route("/api/supervision/approve", post(approve_action))
        .route("/api/supervision/reject", post(reject_action))
        .route("/api/health", get(get_health))
        .route("/api/system/load", get(get_system_load))
        .route("/api/orchestrate", post(orchestrate))
}

async fn get_setting(State(s): State<Arc<ServerState>>, Query(q): Query<KeyQuery>) -> Result<Json<Option<String>>, String> {
    let val = crate::backend::get_setting(&s.db, &q.key).await;
    Ok(Json(Some(val)))
}

async fn set_setting(State(s): State<Arc<ServerState>>, Json(p): Json<KeyVal>) -> Result<(), String> {
    crate::backend::set_setting(&s.db, &p.key, &p.value).await.map_err(|e| e.to_string())
}

async fn get_all_settings(State(s): State<Arc<ServerState>>) -> Result<Json<std::collections::HashMap<String, String>>, String> {
    let settings = crate::backend::get_all_settings(&s.db).await;
    Ok(Json(settings))
}

async fn get_supervision_state(State(s): State<Arc<ServerState>>) -> Result<Json<ComputerSupervisionState>, String> {
    crate::backend::get_computer_supervision_state(&s.db).await.map(Json).map_err(|e| e.to_string())
}

async fn approve_action(State(s): State<Arc<ServerState>>) -> Result<(), String> {
    crate::backend::approve_pending_computer_action(&s.db).await.map_err(|e| e.to_string())
}

async fn reject_action(State(s): State<Arc<ServerState>>) -> Result<(), String> {
    crate::backend::reject_pending_computer_action(&s.db).await.map_err(|e| e.to_string())
}

async fn get_health(State(s): State<Arc<ServerState>>) -> Result<Json<RuntimeHealth>, String> {
    let health = crate::backend::get_runtime_health(&s.db).await;
    Ok(Json(health))
}

async fn get_system_load(State(_s): State<Arc<ServerState>>) -> Result<Json<SystemLoad>, String> {
    Ok(Json(crate::backend::get_system_load().await))
}

async fn orchestrate(
    State(s): State<Arc<ServerState>>,
    Json(p): Json<OrchestratePayload>,
) -> Result<String, String> {
    Ok(s.agent_system.orchestrate(&s.db, p.conversation_id, &p.query).await)
}
