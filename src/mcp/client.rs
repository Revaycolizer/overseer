//! MCP client for connecting to external MCP-compatible servers.
//!
//! Used by the TUI to discover and invoke tools from remote MCP servers.

use std::time::Duration;

use serde_json::Value;

use super::types::*;

/// A client that connects to an MCP server over HTTP.
pub struct McpClient {
    /// Base URL of the MCP server (e.g. "http://localhost:8080/mcp").
    server_url: String,
    /// Shared HTTP client.
    http: reqwest::Client,
    /// Incrementing request ID.
    next_id: u64,
}

impl McpClient {
    /// Create a new MCP client pointing at `server_url`.
    pub fn new(server_url: impl Into<String>) -> Self {
        Self {
            server_url: server_url.into(),
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            next_id: 1,
        }
    }

    /// List all tools exposed by the MCP server.
    pub async fn list_tools(&mut self) -> anyhow::Result<Vec<McpTool>> {
        let resp = self.call(METHOD_TOOLS_LIST, Value::Null).await?;
        let tools = resp
            .get("tools")
            .and_then(|v| serde_json::from_value::<Vec<McpTool>>(v.clone()).ok())
            .unwrap_or_default();
        Ok(tools)
    }

    /// Call a tool on the MCP server.
    pub async fn call_tool(&mut self, name: &str, args: Value) -> anyhow::Result<McpToolResult> {
        let params = serde_json::json!({
            "name": name,
            "arguments": args
        });
        let resp = self.call(METHOD_TOOLS_CALL, params).await?;
        let result = serde_json::from_value::<McpToolResult>(resp)?;
        Ok(result)
    }

    /// List resources from the MCP server.
    pub async fn list_resources(&mut self) -> anyhow::Result<Vec<McpResource>> {
        let resp = self.call(METHOD_RESOURCES_LIST, Value::Null).await?;
        let resources = resp
            .get("resources")
            .and_then(|v| serde_json::from_value::<Vec<McpResource>>(v.clone()).ok())
            .unwrap_or_default();
        Ok(resources)
    }

    /// Read a specific resource by URI.
    pub async fn read_resource(&mut self, uri: &str) -> anyhow::Result<McpReadResourceResult> {
        let params = serde_json::json!({ "uri": uri });
        let resp = self.call(METHOD_RESOURCES_READ, params).await?;
        let result = serde_json::from_value::<McpReadResourceResult>(resp)?;
        Ok(result)
    }

    /// Perform a JSON-RPC call and return the result field.
    async fn call(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;

        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id,
            method: method.into(),
            params,
        };

        let resp = self
            .http
            .post(&self.server_url)
            .json(&request)
            .send()
            .await?;

        let json: JsonRpcResponse = resp.json().await?;

        if let Some(err) = json.error {
            anyhow::bail!("MCP error: {} (code {})", err.message, err.code);
        }

        json.result.ok_or_else(|| anyhow::anyhow!("MCP returned no result"))
    }
}
