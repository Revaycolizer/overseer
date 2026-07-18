use axum::{
    routing::{get, post},
    extract::{State, Json, Path},
    http::StatusCode,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use super::ServerState;

#[derive(Serialize)]
struct Template { id: i64, name: String, description: Option<String>, category: String }

#[derive(Deserialize)]
struct CreateTemplate { name: String, description: Option<String>, category: Option<String>, graph_json: String }

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/workflow-templates", get(list_templates).post(create_template))
        .route("/v1/workflow-templates/{id}/install", post(install_template))
}

async fn list_templates(State(state): State<Arc<ServerState>>) -> Result<Json<Vec<Template>>, String> {
    let rows = sqlx::query_as::<_, (i64, String, Option<String>, String)>(
        "SELECT id, name, description, category FROM workflow_templates ORDER BY category, name",
    ).fetch_all(&state.db.pool).await.map_err(|e| e.to_string())?;
    Ok(Json(rows.into_iter().map(|(id, name, description, category)| Template { id, name, description, category }).collect()))
}

async fn create_template(
    State(state): State<Arc<ServerState>>,
    Json(p): Json<CreateTemplate>,
) -> Result<Json<Template>, (StatusCode, String)> {
    let id = state.db.next_id().await;
    let cat = p.category.unwrap_or_else(|| "custom".into());
    sqlx::query("INSERT INTO workflow_templates (id, name, description, category, graph_json) VALUES (?, ?, ?, ?, ?)")
        .bind(id).bind(&p.name).bind(&p.description).bind(&cat).bind(&p.graph_json)
        .execute(&state.db.pool).await.map_err(|e| (StatusCode::CONFLICT, e.to_string()))?;
    Ok(Json(Template { id, name: p.name, description: p.description, category: cat }))
}

async fn install_template(
    State(state): State<Arc<ServerState>>,
    Path(id): Path<i64>,
) -> Result<Json<i64>, String> {
    let graph: String = sqlx::query_scalar("SELECT graph_json FROM workflow_templates WHERE id = ?")
        .bind(id).fetch_optional(&state.db.pool).await.map_err(|e| e.to_string())?
        .ok_or_else(|| "Template not found".to_string())?;

    let wf_id = state.db.next_id().await;
    sqlx::query("INSERT INTO workflows (id, name, status) VALUES (?, ?, 'draft')")
        .bind(wf_id).bind(format!("From Template {}", id))
        .execute(&state.db.pool).await.map_err(|e| e.to_string())?;

    let data: serde_json::Value = serde_json::from_str(&graph).map_err(|e| e.to_string())?;
    if let Some(nodes) = data.get("nodes").and_then(|v| v.as_array()) {
        for node in nodes {
            let nid = state.db.next_id().await;
            sqlx::query("INSERT INTO workflow_nodes (id, workflow_id, node_type, label, config_json) VALUES (?, ?, ?, ?, ?)")
                .bind(nid).bind(wf_id)
                .bind(node.get(1).and_then(|v| v.as_str()).unwrap_or("manual_trigger"))
                .bind(node.get(2).and_then(|v| v.as_str()).unwrap_or("Node"))
                .bind(node.get(3).and_then(|v| v.as_str()).unwrap_or("{}"))
                .execute(&state.db.pool).await.map_err(|e| e.to_string())?;
        }
    }

    Ok(Json(wf_id))
}
