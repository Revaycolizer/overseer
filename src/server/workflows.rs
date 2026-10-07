use axum::{
    routing::{get, post},
    extract::{Path, State, Json},
    http::StatusCode,
    Router,
};
use std::sync::Arc;
use serde::{Deserialize, Serialize};
use super::ServerState;
use crate::backend::{Workflow, WorkflowStep, WorkflowRun, WorkflowStepRun};

#[derive(Deserialize)]
pub struct AddWorkflowPayload { pub name: String }

#[derive(Deserialize)]
pub struct DeleteWorkflowPayload { pub id: i64 }

#[derive(Serialize)]
struct WorkflowVersion { id: i64, version: i32, created_at: String }

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/workflows", get(get_workflows).post(add_workflow))
        .route("/api/workflows/update", post(update_workflow))
        .route("/api/workflows/delete", post(delete_workflow))
        .route("/api/workflows/:id/steps", get(get_workflow_steps).post(add_workflow_step))
        .route("/api/workflows/steps/update", post(update_workflow_step))
        .route("/api/workflows/steps/delete", post(delete_workflow_step))
        .route("/api/workflows/:id/runs", get(get_workflow_runs))
        .route("/api/workflows/runs/:id/steps", get(get_workflow_step_runs))
        .route("/api/workflows/:id/run", post(run_workflow))
        .route("/v1/workflows/:id/publish", post(publish_workflow))
        .route("/v1/workflows/:id/versions", get(get_versions))
        .route("/v1/workflows/:id/versions/:v/restore", post(restore_version))
}

async fn get_workflows(State(state): State<Arc<ServerState>>) -> Result<Json<Vec<Workflow>>, String> {
    crate::backend::workflows::crud::get_workflows(&state.db).await.map(Json).map_err(|e| e.to_string())
}

async fn add_workflow(State(state): State<Arc<ServerState>>, Json(p): Json<AddWorkflowPayload>) -> Result<Json<i64>, String> {
    crate::backend::workflows::crud::add_workflow(&state.db, &p.name).await.map(Json).map_err(|e| e.to_string())
}

async fn update_workflow(State(state): State<Arc<ServerState>>, Json(w): Json<Workflow>) -> Result<(), String> {
    crate::backend::workflows::crud::update_workflow(&state.db, w).await.map_err(|e| e.to_string())
}

async fn delete_workflow(State(state): State<Arc<ServerState>>, Json(p): Json<DeleteWorkflowPayload>) -> Result<(), String> {
    crate::backend::workflows::crud::delete_workflow(&state.db, p.id).await.map_err(|e| e.to_string())
}

async fn get_workflow_steps(State(state): State<Arc<ServerState>>, Path(id): Path<i64>) -> Result<Json<Vec<WorkflowStep>>, String> {
    crate::backend::workflows::crud::get_workflow_steps(&state.db, id).await.map(Json).map_err(|e| e.to_string())
}

async fn add_workflow_step(State(state): State<Arc<ServerState>>, Path(id): Path<i64>) -> Result<Json<i64>, String> {
    crate::backend::workflows::crud::add_workflow_step(&state.db, id).await.map(Json).map_err(|e| e.to_string())
}

async fn update_workflow_step(State(state): State<Arc<ServerState>>, Json(s): Json<WorkflowStep>) -> Result<(), String> {
    crate::backend::workflows::crud::update_workflow_step(&state.db, s).await.map_err(|e| e.to_string())
}

async fn delete_workflow_step(State(state): State<Arc<ServerState>>, Json(p): Json<DeleteWorkflowPayload>) -> Result<(), String> {
    crate::backend::workflows::crud::delete_workflow_step(&state.db, p.id).await.map_err(|e| e.to_string())
}

async fn get_workflow_runs(State(state): State<Arc<ServerState>>, Path(id): Path<i64>) -> Result<Json<Vec<WorkflowRun>>, String> {
    crate::backend::workflows::crud::get_workflow_runs(&state.db, id).await.map(Json).map_err(|e| e.to_string())
}

async fn get_workflow_step_runs(State(state): State<Arc<ServerState>>, Path(id): Path<i64>) -> Result<Json<Vec<WorkflowStepRun>>, String> {
    crate::backend::workflows::crud::get_workflow_step_runs(&state.db, id).await.map(Json).map_err(|e| e.to_string())
}

async fn run_workflow(State(state): State<Arc<ServerState>>, Path(id): Path<i64>) -> Result<Json<i64>, String> {
    crate::backend::workflows::run::run_workflow(&state.db, &state.agent_system, id).await.map(Json).map_err(|e| e.to_string())
}

async fn publish_workflow(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<()>, (StatusCode, String)> {
    let nodes = sqlx::query_as::<_, (i64, String, String, String, f64, f64)>(
        "SELECT id, node_type, label, config_json, position_x, position_y FROM workflow_nodes WHERE workflow_id = ?",
    ).bind(id).fetch_all(&state.db.pool).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let edges = sqlx::query_as::<_, (i64, i64, i64, String, String)>(
        "SELECT id, source_node, target_node, source_port, target_port FROM workflow_edges WHERE workflow_id = ?",
    ).bind(id).fetch_all(&state.db.pool).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let graph_json = serde_json::json!({"nodes": nodes, "edges": edges}).to_string();
    let max_v: Option<(i32,)> = sqlx::query_as("SELECT COALESCE(MAX(version), 0) FROM workflow_versions WHERE workflow_id = ?")
        .bind(id).fetch_optional(&state.db.pool).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let version = max_v.map(|v| v.0 + 1).unwrap_or(1);
    let vid = state.db.next_id().await;

    sqlx::query("INSERT INTO workflow_versions (id, workflow_id, version, graph_json) VALUES (?, ?, ?, ?)")
        .bind(vid).bind(id).bind(version).bind(&graph_json)
        .execute(&state.db.pool).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    sqlx::query("UPDATE workflows SET status = 'published', published_at = datetime('now') WHERE id = ?")
        .bind(id).execute(&state.db.pool).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(()))
}

async fn get_versions(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<WorkflowVersion>>, String> {
    let rows = sqlx::query_as::<_, (i64, i32, String)>(
        "SELECT id, version, created_at FROM workflow_versions WHERE workflow_id = ? ORDER BY version DESC",
    ).bind(id).fetch_all(&state.db.pool).await.map_err(|e| e.to_string())?;
    Ok(Json(rows.into_iter().map(|(id, version, created_at)| WorkflowVersion { id, version, created_at }).collect()))
}

async fn restore_version(
    State(state): State<Arc<ServerState>>,
    Path((id, v)): Path<(i64, i32)>,
) -> Result<Json<()>, String> {
    let graph: Option<(String,)> = sqlx::query_as(
        "SELECT graph_json FROM workflow_versions WHERE workflow_id = ? AND version = ?",
    ).bind(id).bind(v).fetch_optional(&state.db.pool).await.map_err(|e| e.to_string())?;
    let Some((json,)) = graph else { return Err("Version not found".into()) };

    let data: serde_json::Value = serde_json::from_str(&json).map_err(|e| e.to_string())?;
    if let Some(nodes) = data.get("nodes") {
        for node in nodes.as_array().unwrap_or(&vec![]) {
            if let (Some(nid), Some(nt), Some(nl), Some(cfg), Some(px), Some(py)) = (
                node.get(0).and_then(|v| v.as_i64()),
                node.get(1).and_then(|v| v.as_str()),
                node.get(2).and_then(|v| v.as_str()),
                node.get(3).and_then(|v| v.as_str()),
                node.get(4).and_then(|v| v.as_f64()),
                node.get(5).and_then(|v| v.as_f64()),
            ) {
                sqlx::query("INSERT OR REPLACE INTO workflow_nodes (id, workflow_id, node_type, label, config_json, position_x, position_y) VALUES (?, ?, ?, ?, ?, ?, ?)")
                    .bind(nid).bind(id).bind(nt).bind(nl).bind(cfg).bind(px).bind(py)
                    .execute(&state.db.pool).await.map_err(|e| e.to_string())?;
            }
        }
    }
    Ok(Json(()))
}
