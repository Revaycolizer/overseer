use axum::{
    routing::get,
    extract::{State, Json},
    Router,
};
use std::sync::Arc;
use super::ServerState;
use crate::backend::*;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/api/planners", get(get_planners).post(add_planner))
}

async fn get_planners(State(s): State<Arc<ServerState>>) -> Result<Json<Vec<Planner>>, String> {
    crate::backend::get_planners(&s.db).await.map(Json).map_err(|e| e.to_string())
}

async fn add_planner(State(s): State<Arc<ServerState>>, Json(p): Json<serde_json::Value>) -> Result<Json<i64>, String> {
    let name = p["name"].as_str().unwrap_or("");
    let inst = p["instructions"].as_str().unwrap_or("");
    let model = p["model"].as_str().unwrap_or("");
    crate::backend::add_planner(&s.db, name, inst, model).await.map(Json).map_err(|e| e.to_string())
}
