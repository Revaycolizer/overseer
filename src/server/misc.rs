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
        .route("/health", get(get_health))
        .route("/api/health", get(get_health))
        .route("/api/system/load", get(get_system_load))
        .route("/api/system/open_path", post(open_path))
        .route("/api/system/export", post(export))
        .route("/api/system/save_chart", post(save_chart))
        .route("/api/system/test_preamble", post(test_preamble))
        .route("/api/registry/refresh", post(refresh_registry))
        .route("/api/registry/status", get(get_registry_status))
        .route("/api/models/available", get(get_models))
        .route("/api/orchestrate", post(orchestrate))
}

async fn get_setting(State(s): State<Arc<ServerState>>, Query(q): Query<KeyQuery>) -> Result<Json<Option<String>>, String> {
    Ok(Json(Some(crate::backend::get_setting(&s.db, &q.key).await)))
}
async fn set_setting(State(s): State<Arc<ServerState>>, Json(p): Json<KeyVal>) -> Result<(), (axum::http::StatusCode, String)> {
    crate::backend::set_setting(&s.db, &p.key, &p.value).await.map_err(|e| (axum::http::StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
}
async fn get_all_settings(State(s): State<Arc<ServerState>>) -> Result<Json<std::collections::HashMap<String, String>>, String> {
    Ok(Json(crate::backend::get_all_settings(&s.db).await))
}
async fn get_health(State(s): State<Arc<ServerState>>) -> Result<Json<RuntimeHealth>, String> {
    Ok(Json(crate::backend::get_runtime_health(&s.db).await))
}
async fn get_system_load(State(_s): State<Arc<ServerState>>) -> Result<Json<SystemLoad>, String> {
    Ok(Json(crate::backend::get_system_load().await))
}
async fn open_path(State(_s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<(), String> {
    let path = p["path"].as_str().unwrap_or("");
    crate::backend::open_local_path(path).await.map_err(|e| e.to_string())
}
async fn export(State(_s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<Json<String>, String> {
    let name = p["filename"].as_str().unwrap_or("");
    let content = p["content"].as_str().unwrap_or("");
    let download_dir = dirs::download_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("assets/exports"))
        .join("Overseer");
    if !download_dir.exists() {
        let _ = std::fs::create_dir_all(&download_dir);
    }
    let path = download_dir.join(name);
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(Json(path.to_string_lossy().to_string()))
}
async fn save_chart(State(_s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<Json<String>, String> {
    let ch = p["chart_json"].as_str().unwrap_or("");
    let val: serde_json::Value = serde_json::from_str(ch).unwrap_or_default();
    let title = val.get("title").and_then(|v| v.as_str()).unwrap_or("chart").to_string();
    let data = val.get("data").and_then(|v| v.as_str()).unwrap_or(ch).to_string();
    let format = val.get("format").and_then(|v| v.as_str()).unwrap_or("svg").to_string();
    crate::backend::save_chart_export(title, data, format).await.map(Json).map_err(|e| e.to_string())
}
async fn test_preamble(State(_s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<Json<String>, String> {
    let pr = p["preamble"].as_str().unwrap_or("");
    crate::backend::test_agent_preamble(pr).await.map(Json).map_err(|e| e.to_string())
}
async fn refresh_registry(State(s): State<Arc<ServerState>>) -> Result<(), String> {
    crate::backend::refresh_runtime_registry_now(&s.db).await.map_err(|e| e.to_string())
}
async fn get_registry_status(State(s): State<Arc<ServerState>>) -> Result<Json<RuntimeRegistryStatus>, String> {
    Ok(Json(crate::backend::get_runtime_registry_status(&s.db).await))
}
async fn get_models(State(_s): State<Arc<ServerState>>) -> Result<Json<Vec<String>>, (axum::http::StatusCode, String)> {
    crate::backend::try_get_available_models().await.map(Json)
        .map_err(|e| (axum::http::StatusCode::SERVICE_UNAVAILABLE, format!("Ollama unavailable: {e}")))
}
async fn orchestrate(State(s): State<Arc<ServerState>>, Json(p): Json<OrchestratePayload>) -> Result<String, String> {
    Ok(s.agent_system.orchestrate(&s.db, p.conversation_id, &p.query, None).await)
}
