use axum::{
    routing::{get, post},
    extract::{Path, State, Json},
    Router,
};
use std::sync::Arc;
use serde::Deserialize;
use super::ServerState;
use crate::backend::{Workflow, WorkflowStep, WorkflowRun, WorkflowStepRun};

#[derive(Deserialize)]
pub struct AddWorkflowPayload {
    pub name: String,
}

#[derive(Deserialize)]
pub struct DeleteWorkflowPayload {
    pub id: i64,
}

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
