//! Explicit, tool-free provider check used by desktop first-run setup.
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use serde::Deserialize;
use std::sync::Arc;
use super::ServerState;

#[derive(Deserialize)]
struct Check { provider: String, model: String, #[serde(default)] api_key: String }
pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/api/setup/test", post(test))
}
async fn test(State(state): State<Arc<ServerState>>, Json(check): Json<Check>) -> Result<Json<serde_json::Value>, (StatusCode, String)> {
    if !matches!(check.provider.as_str(), "ollama" | "openai" | "anthropic") || check.model.trim().is_empty() || check.model.len()>256 {
        return Err((StatusCode::BAD_REQUEST,"Choose a supported provider and a model.".into()));
    }
    let result = tokio::time::timeout(std::time::Duration::from_secs(45), async {
        if check.provider == "ollama" {
            let models = crate::backend::try_get_available_models().await.map_err(anyhow::Error::msg)?;
            anyhow::ensure!(models.iter().any(|name| name == &check.model), "Install the selected model in Ollama first.");
            let response = reqwest::Client::new().post("http://localhost:11434/api/chat")
                .json(&serde_json::json!({"model":check.model,"stream":false,"think":false,"messages":[{"role":"user","content":"Reply with OK."}],"options":{"num_predict":16}}))
                .send().await?.error_for_status()?.json::<serde_json::Value>().await?;
            anyhow::ensure!(response["message"]["content"].as_str().is_some_and(|s| !s.trim().is_empty()), "Model returned no text.");
        } else {
            let key = if check.api_key.trim().is_empty() { crate::backend::get_setting(&state.db, &format!("{}_api_key", check.provider)).await } else { check.api_key.trim().to_string() };
            anyhow::ensure!(!key.is_empty(), "Enter your provider API key.");
            let base = crate::backend::get_setting_or(&state.db, &format!("{}_base_url", check.provider), if check.provider=="openai" {"https://api.openai.com/v1"} else {"https://api.anthropic.com"}).await;
            let config = crate::engine::providers::ProviderConfig {provider:crate::engine::providers::Provider::parse(&check.provider),api_key:key,base_url:base.trim_end_matches('/').to_string(),model:check.model.trim().to_string()};
            let text = crate::engine::providers::complete_chat(&config,"This is a connection check. Reply briefly.","Reply with OK.").await?;
            anyhow::ensure!(!text.trim().is_empty(), "Model returned no text.");
        }
        Ok::<(),anyhow::Error>(())
    }).await;
    match result {
        Ok(Ok(())) => Ok(Json(serde_json::json!({"ok":true}))),
        Ok(Err(_)) => Err((StatusCode::BAD_GATEWAY,"The model check failed. Check that the model is available, your API key is valid, and your provider account has access. For local AI, make sure Ollama is running.".into())),
        Err(_) => Err((StatusCode::GATEWAY_TIMEOUT,"The model took too long to respond. Try again or choose a smaller local model.".into())),
    }
}
