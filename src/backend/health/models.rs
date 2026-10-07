use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

struct AvailableModelsCache {
    fetched_at: Instant,
    models: Vec<String>,
}

static AVAILABLE_MODELS_CACHE: OnceLock<Mutex<Option<AvailableModelsCache>>> = OnceLock::new();
const AVAILABLE_MODELS_CACHE_TTL: Duration = Duration::from_secs(8);

pub async fn get_available_models() -> Vec<String> {
    let cache = AVAILABLE_MODELS_CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(guard) = cache.lock() {
        if let Some(entry) = guard.as_ref() {
            if entry.fetched_at.elapsed() < AVAILABLE_MODELS_CACHE_TTL {
                return entry.models.clone();
            }
        }
    }

    // Never show fabricated installed models when Ollama is unavailable.
    let models = try_get_available_models().await.unwrap_or_default();

    if let Ok(mut guard) = cache.lock() {
        *guard = Some(AvailableModelsCache {
            fetched_at: Instant::now(),
            models: models.clone(),
        });
    }

    models
}

pub(crate) async fn try_get_available_models() -> Result<Vec<String>, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|err| err.to_string())?
        .error_for_status()
        .map_err(|err| err.to_string())?;

    let json = response
        .json::<serde_json::Value>()
        .await
        .map_err(|err| err.to_string())?;

    Ok(json["models"]
        .as_array()
        .map(|models| {
            models
                .iter()
                .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default())
}
