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

#[derive(Deserialize)]
pub struct RunIDQuery { pub execution_run_id: i64 }

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/artifacts", get(get_artifacts).post(create_artifact))
        .route("/api/artifacts/by_ids", post(get_artifacts_by_ids))
        .route("/api/artifacts/by_run", get(get_artifacts_for_execution_run))
}

async fn get_artifacts(State(s): State<Arc<ServerState>>, Query(q): Query<CIDQuery>) -> Result<Json<Vec<Artifact>>, String> {
    crate::backend::artifacts::get_artifacts_for_conversation(&s.db, q.conversation_id, 50).await.map(Json).map_err(|e| e.to_string())
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
    crate::backend::artifacts::create_artifact(&s.db, cid, execution_run_id, domain, step, kind, label, content, fp, ct, m).await.map(Json).map_err(|e| e.to_string())
}

async fn get_artifacts_by_ids(State(s): State<Arc<ServerState>>, Json(ids): Json<Vec<i64>>) -> Result<Json<Vec<Artifact>>, String> {
    crate::backend::artifacts::get_artifacts_by_ids(&s.db, &ids).await.map(Json).map_err(|e| e.to_string())
}

async fn get_artifacts_for_execution_run(State(s): State<Arc<ServerState>>, Query(q): Query<RunIDQuery>) -> Result<Json<Vec<Artifact>>, String> {
    crate::backend::artifacts::get_artifacts_for_execution_run(&s.db, q.execution_run_id).await.map(Json).map_err(|e| e.to_string())
}
