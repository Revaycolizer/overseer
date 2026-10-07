//! MCP JSON-RPC 2.0 server over HTTP.
//!
//! Mount at `/mcp` on the main Axum server. Exposes:
//! - `tools/list` — all registered tools
//! - `tools/call` — invoke a tool by name with arguments
//! - `resources/list` — agents and supervisors as resources
//! - `resources/read` — get agent/supervisor details

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde_json::Value;

use crate::server::ServerState;

use super::types::*;

/// Returns an Axum router with MCP endpoints, using the shared ServerState.
pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/mcp", post(handle_mcp_request))
}

async fn handle_mcp_request(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<JsonRpcRequest>,
) -> Result<Json<JsonRpcResponse>, (StatusCode, String)> {
    let id = req.id;

    let response = match req.method.as_str() {
        METHOD_TOOLS_LIST => handle_tools_list(&state, id).await,
        METHOD_TOOLS_CALL => handle_tools_call(&state, id, &req.params).await,
        METHOD_RESOURCES_LIST => handle_resources_list(&state, id).await,
        METHOD_RESOURCES_READ => handle_resources_read(&state, id, &req.params).await,
        _ => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: ERROR_METHOD_NOT_FOUND,
                message: format!("Method '{}' not found", req.method),
                data: None,
            }),
        },
    };

    Ok(Json(response))
}

async fn handle_tools_list(state: &Arc<ServerState>, id: u64) -> JsonRpcResponse {
    let tools = crate::backend::get_tools(&state.db)
        .await
        .unwrap_or_default();

    let mcp_tools: Vec<McpTool> = tools
        .iter()
        .map(|t| McpTool {
            name: t.name.clone(),
            description: t.description.clone(),
            input_schema: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "input": { "type": "string", "description": "The input for the tool" }
                }
            })),
        })
        .collect();

    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(serde_json::json!({ "tools": mcp_tools })),
        error: None,
    }
}

async fn handle_tools_call(state: &Arc<ServerState>, id: u64, params: &Value) -> JsonRpcResponse {
    let _name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let _arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));

    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(serde_json::json!({
            "content": [{
                "type": "text",
                "text": format!("Tool call received by MCP server. Full kernel_execute integration pending.")
            }]
        })),
        error: None,
    }
}

async fn handle_resources_list(state: &Arc<ServerState>, id: u64) -> JsonRpcResponse {
    let agents = crate::backend::get_agents(&state.db)
        .await
        .unwrap_or_default();
    let supervisors = crate::backend::get_supervisors(&state.db)
        .await
        .unwrap_or_default();

    let mut resources = Vec::new();

    for agent in &agents {
        resources.push(McpResource {
            uri: format!("agent://{}", agent.domain),
            name: agent.name.clone(),
            description: Some(agent.description.clone()),
            mime_type: Some("application/json".into()),
        });
    }

    for sup in &supervisors {
        resources.push(McpResource {
            uri: format!("supervisor://{}", sup.domain),
            name: sup.name.clone(),
            description: sup.description.clone(),
            mime_type: Some("application/json".into()),
        });
    }

    JsonRpcResponse {
        jsonrpc: "2.0".into(),
        id,
        result: Some(serde_json::json!({ "resources": resources })),
        error: None,
    }
}

async fn handle_resources_read(
    state: &Arc<ServerState>,
    id: u64,
    params: &Value,
) -> JsonRpcResponse {
    let uri = params.get("uri").and_then(|v| v.as_str()).unwrap_or("");

    let contents = if let Some(domain) = uri.strip_prefix("agent://") {
        let agents = crate::backend::get_agents(&state.db)
            .await
            .unwrap_or_default();
        agents
            .iter()
            .find(|a| a.domain == domain)
            .map(|a| serde_json::to_string_pretty(a).unwrap_or_default())
    } else if let Some(domain) = uri.strip_prefix("supervisor://") {
        let supervisors = crate::backend::get_supervisors(&state.db)
            .await
            .unwrap_or_default();
        supervisors
            .iter()
            .find(|s| s.domain == domain)
            .map(|s| serde_json::to_string_pretty(s).unwrap_or_default())
    } else {
        None
    };

    match contents {
        Some(text) => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(
                serde_json::json!({ "contents": [{ "uri": uri, "mime_type": "application/json", "text": text }] }),
            ),
            error: None,
        },
        None => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code: ERROR_INVALID_PARAMS,
                message: format!("Resource '{}' not found", uri),
                data: None,
            }),
        },
    }
}
