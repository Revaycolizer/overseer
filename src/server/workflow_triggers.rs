use axum::{
    routing::post,
    extract::{State, Json, Path},
    http::StatusCode,
    Router,
};
use serde_json::Value;
use std::sync::Arc;
use super::ServerState;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/v1/workflows/webhook/{workflow_id}/{token}", post(webhook_trigger))
}

async fn webhook_trigger(
    State(state): State<Arc<ServerState>>,
    Path((workflow_id, token)): Path<(i64, String)>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, StatusCode> {
    let nodes = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, node_type, config_json FROM workflow_nodes WHERE workflow_id = ?",
    )
    .bind(workflow_id)
    .fetch_all(&state.db.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let graph = crate::engine::workflow::graph::WorkflowGraph {
        nodes: nodes.into_iter().map(|(id, node_type, config_json)| {
            crate::engine::workflow::graph::WorkflowNode {
                id, workflow_id, node_type, label: String::new(), config_json,
            }
        }).collect(),
        edges: vec![],
    };

    let runner = crate::engine::workflow::runner::WorkflowRunner::new(graph, state.db.clone());
    let results = runner.execute().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(serde_json::json!({"results": results})))
}
