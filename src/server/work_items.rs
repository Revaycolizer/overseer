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
pub struct CIDQuery { pub conversation_id: i64 }

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/work_items", get(get_work_items).post(create_work_item))
        .route("/api/artifacts", get(get_artifacts).post(create_artifact))
}

async fn get_work_items(State(s): State<Arc<ServerState>>, Query(q): Query<CIDQuery>) -> Result<Json<Vec<WorkItem>>, String> {
    crate::backend::get_work_items_for_conversation(&s.db, q.conversation_id, 100).await.map(Json).map_err(|e| e.to_string())
}

async fn create_work_item(State(s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<Json<i64>, String> {
    let cid = p["conversation_id"].as_i64().unwrap_or(0);
    let title = p["title"].as_str().unwrap_or("");
    let brief = p["brief"].as_str();
    let status = p["status"].as_str().unwrap_or("queued");
    let kind = p["surface_kind"].as_str().unwrap_or("chat");
    let domain = p["assigned_domain"].as_str();
    let prompt = p["prompt"].as_str().unwrap_or("");
    let sa = p["source_artifact_ids_json"].as_str();
    let m = p["metadata_json"].as_str();
    crate::backend::create_work_item(&s.db, cid, title, brief, status, kind, domain, prompt, sa, m).await.map(Json).map_err(|e| e.to_string())
}

async fn get_artifacts(State(s): State<Arc<ServerState>>, Query(q): Query<CIDQuery>) -> Result<Json<Vec<Artifact>>, String> {
    crate::backend::get_artifacts_for_conversation(&s.db, q.conversation_id, 50).await.map(Json).map_err(|e| e.to_string())
}

async fn create_artifact(State(s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<Json<i64>, String> {
    let cid = p["conversation_id"].as_i64().unwrap_or(0);
    let execution_run_id = p["execution_run_id"].as_i64();
    let domain = p["domain"].as_str();
    let step = p["step_text"].as_str();
    let kind = p["kind"].as_str().unwrap_or("");
    let label = p["label"].as_str().unwrap_or("");
    let content = p["content"].as_str();
    let fp = p["file_path"].as_str();
    let ct = p["content_type"].as_str();
    let m = p["metadata_json"].as_str();
    crate::backend::create_artifact(&s.db, cid, execution_run_id, domain, step, kind, label, content, fp, ct, m).await.map(Json).map_err(|e| e.to_string())
}
