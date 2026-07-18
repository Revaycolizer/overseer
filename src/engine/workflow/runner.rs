use std::sync::Arc;
use crate::db::Database;
use serde_json::Value;
use super::graph::WorkflowGraph;

pub struct WorkflowRunner {
    pub graph: WorkflowGraph,
    pub db: Arc<Database>,
}

impl WorkflowRunner {
    pub fn new(graph: WorkflowGraph, db: Arc<Database>) -> Self {
        Self { graph, db }
    }

    pub async fn execute(&self) -> anyhow::Result<Vec<(i64, serde_json::Value)>> {
        let order = self.graph.topological_sort()
            .map_err(|_| anyhow::anyhow!("Workflow contains a cycle"))?;
        let mut results = Vec::new();

        for node_id in order {
            let node = self.graph.nodes.iter().find(|n| n.id == node_id)
                .ok_or_else(|| anyhow::anyhow!("Node {node_id} not found"))?;
            let output = self.execute_node(node).await?;
            results.push((node_id, output));
        }

        Ok(results)
    }

    async fn execute_node(&self, node: &super::graph::WorkflowNode) -> anyhow::Result<serde_json::Value> {
        let cfg: Value = serde_json::from_str(&node.config_json).unwrap_or(Value::Object(Default::default()));
        match node.node_type.as_str() {
            "manual_trigger" => Ok(serde_json::json!({"triggered": true})),
            "agent" => {
                let query = cfg.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let db = self.db.clone();
                let system = crate::engine::system::AgentSystem::new((*db).clone());
                Ok(serde_json::json!({"response": system.orchestrate(&db, 0, query, None).await}))
            }
            "llm" => {
                let prompt = cfg.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
                let model = cfg.get("model").and_then(|v| v.as_str()).unwrap_or("qwen2.5:7b-instruct");
                let body = serde_json::json!({"model": model, "prompt": prompt, "stream": false});
                let resp = reqwest::Client::new()
                    .post("http://localhost:11434/api/generate")
                    .json(&body).send().await.map_err(|e| anyhow::anyhow!("ollama: {e}"))?;
                let data: Value = resp.json().await.map_err(|e| anyhow::anyhow!("ollama parse: {e}"))?;
                let text = data.get("response").and_then(|v| v.as_str()).unwrap_or("").to_string();
                Ok(serde_json::json!({"text": text}))
            }
            "set" => Ok(cfg),
            "http_request" => {
                let url = cfg.get("url").and_then(|v| v.as_str()).ok_or_else(|| anyhow::anyhow!("http_request: missing url"))?;
                let method = cfg.get("method").and_then(|v| v.as_str()).unwrap_or("GET");
                let headers: Value = cfg.get("headers").cloned().unwrap_or(Value::Object(Default::default()));
                let body = cfg.get("body");
                let client = reqwest::Client::new();
                let req = match method {
                    "POST" => client.post(url).json(&body),
                    "PUT" => client.put(url).json(&body),
                    "DELETE" => client.delete(url),
                    _ => client.get(url),
                };
                let resp = req.headers(reqwest::header::HeaderMap::new()).send().await.map_err(|e| anyhow::anyhow!("http: {e}"))?;
                let status = resp.status().as_u16();
                let text = resp.text().await.unwrap_or_default();
                Ok(serde_json::json!({"status": status, "body": text}))
            }
            "if" => {
                let condition = cfg.get("condition").and_then(|v| v.as_str()).unwrap_or("");
                Ok(serde_json::json!({"condition": condition, "result": !condition.is_empty()}))
            }
            _ => Ok(serde_json::json!({"error": format!("Unknown node type: {}", node.node_type)})),
        }
    }
}
