use axum::{
    routing::{get, post},
    extract::{State, Json},
    Router,
};
use std::sync::Arc;
use super::ServerState;
use crate::backend::*;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/supervision/state", get(get_supervision_state))
        .route("/api/supervision/approve", post(approve_action))
        .route("/api/supervision/reject", post(reject_action))
        .route("/api/supervision/activate", post(activate_session))
        .route("/api/supervision/clear_pending", post(clear_pending))
        .route("/api/supervision/stage", post(stage_request))
        .route("/api/supervision/register_execution", post(register_execution))
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

async fn activate_session(State(s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<(), String> {
    let en = p["enabled"].as_bool().unwrap_or(false);
    let al = p["allowlist"].as_str().unwrap_or("");
    let state_str = if en { "observing" } else { "idle" };
    crate::backend::activate_computer_session(&s.db, state_str, al).await.map_err(|e| e.to_string())
}

async fn clear_pending(State(s): State<Arc<ServerState>>) -> Result<(), String> {
    crate::backend::clear_computer_pending_action(&s.db).await.map_err(|e| e.to_string())
}

async fn stage_request(State(s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<(), String> {
    let tool_name = p["tool_name"].as_str().unwrap_or("");
    let input_json = p["input_json"].as_str().unwrap_or("");
    let val: serde_json::Value = serde_json::from_str(input_json).unwrap_or_default();
    let summary = crate::engine::orchestrator::dispatch::tools::summarize_computer_action(tool_name, &val);
    crate::backend::stage_computer_approval_request(&s.db, "computer", tool_name, &summary).await.map_err(|e| e.to_string())
}

async fn register_execution(State(s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<(), String> {
    let output_json = p["output_json"].as_str().unwrap_or("");
    let val: serde_json::Value = serde_json::from_str(output_json).unwrap_or_default();
    let tool_name = val.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
    let summary = val.get("summary").and_then(|v| v.as_str()).unwrap_or(output_json);
    crate::backend::register_computer_action_execution(&s.db, tool_name, summary).await.map(|_| ()).map_err(|e| e.to_string())
}
