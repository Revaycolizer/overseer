use axum::{
    routing::{get, post},
    extract::{Query, State, Json, Path},
    Router,
};
use std::sync::Arc;
use serde::Deserialize;
use super::ServerState;
use crate::backend::*;

#[derive(Deserialize)]
pub struct CIDQuery { pub conversation_id: i64 }

#[derive(Deserialize)]
pub struct LimitQuery { pub limit: i64 }

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/work_items", get(get_work_items).post(create_work_item))
        .route("/api/work_items/update_status", post(update_status))
        .route("/api/work_items/update_runtime", post(update_runtime))
        .route("/api/work_items/event", post(log_event))
        .route("/api/work_items/:id/events", get(get_events))
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

async fn update_status(State(s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<(), String> {
    let id = p["id"].as_i64().unwrap_or(0);
    let status = p["status"].as_str().unwrap_or("");
    crate::backend::work_items::update_work_item_status(&s.db, id, status).await.map_err(|e| e.to_string())
}

async fn update_runtime(State(s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<(), String> {
    let id = p["id"].as_i64().unwrap_or(0);
    let status = p["status"].as_str().unwrap_or("");
    let m = p["metadata_json"].as_str();
    crate::backend::work_items::update_work_item_runtime(&s.db, id, status, m).await.map_err(|e| e.to_string())
}

async fn log_event(State(s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<Json<i64>, String> {
    let wid = p["work_item_id"].as_i64().unwrap_or(0);
    let rid = p["execution_run_id"].as_i64();
    let t = p["event_type"].as_str().unwrap_or("");
    let status = p["status"].as_str();
    let sm = p["summary"].as_str();
    let m = p["metadata_json"].as_str();
    crate::backend::work_items::log_work_item_event(&s.db, wid, rid, t, status, sm, m).await.map(Json).map_err(|e| e.to_string())
}

async fn get_events(State(s): State<Arc<ServerState>>, Path(id): Path<i64>, Query(q): Query<LimitQuery>) -> Result<Json<Vec<WorkItemEvent>>, String> {
    crate::backend::work_items::get_work_item_events(&s.db, id, q.limit).await.map(Json).map_err(|e| e.to_string())
}
