use std::sync::Arc;
use crate::db::Database;
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
        match node.node_type.as_str() {
            "manual_trigger" => Ok(serde_json::json!({"triggered": true})),
            "agent" => {
                let query = node.config_json.get("query").and_then(|v| v.as_str()).unwrap_or("");
                let system = crate::engine::system::AgentSystem::new(self.db.as_ref().clone());
                let response = system.orchestrate(&self.db, 0, query, None).await;
                Ok(serde_json::json!({"response": response}))
            }
            "llm" => {
                let prompt = node.config_json.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
                let model = node.config_json.get("model").and_then(|v| v.as_str()).unwrap_or("qwen2.5:7b-instruct");
                let client = rig::providers::ollama::Client::new();
                let agent = rig::agent::AgentBuilder::new(client.completion_model(model))
                    .preamble("You are a helpful assistant.").build();
                match agent.prompt(prompt).await {
                    Ok(text) => Ok(serde_json::json!({"text": text})),
                    Err(e) => Ok(serde_json::json!({"error": e.to_string()})),
                }
            }
            "set" => Ok(serde_json::json!(node.config_json)),
            _ => Ok(serde_json::json!({"error": format!("Unknown node type: {}", node.node_type)})),
        }
    }
}
