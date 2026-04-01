#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum OrchestratorError {
    DatabaseError(String),
    ModelError(String),
    Timeout,
    MaxIterationsReached,
    AgentNotFound(String),
    ToolError(String),
}

pub fn user_facing_runtime_error(details: &str) -> Option<String> {
    let lower = details.to_lowercase();
    let ollama_related = lower.contains("localhost:11434") || lower.contains("/api/chat") || lower.contains("/api/generate") || lower.contains("/api/embed") || lower.contains("ollama");
    let connection_related = lower.contains("error sending request") || lower.contains("connection refused") || lower.contains("failed to connect") || lower.contains("http client error") || lower.contains("tcp connect error") || lower.contains("dns error");

    if ollama_related || connection_related {
        return Some("Model backend unavailable. Overseer could not reach Ollama on `http://localhost:11434`.\n\nTry this:\n1. Start Ollama with `ollama serve`\n2. Check it responds with `ollama list`\n3. Pull a model (e.g., `ollama pull qwen3:8b`)\n\nAfter Ollama is healthy, send the message again.".to_string());
    }
    None
}
