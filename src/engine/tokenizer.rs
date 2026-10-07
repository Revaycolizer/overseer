/// Token-counting helpers with dynamic model context-window detection.
///
/// Uses a 4-characters-per-token rule of thumb which is close enough
/// for the qwen/llama model family Overseer ships with. The real
/// tokenizer for each model family lives on Ollama's side; this
/// helper exists so the UI can show a meaningful "N tokens" badge
/// and so the orchestrator can warn before a long conversation
/// crosses the model's `num_ctx` budget.
///
/// Swap in `tiktoken-rs` later if model-specific accuracy becomes
/// important. The estimator stays as a fast fallback either way.
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

/// Cache of model -> context window size (in tokens).
static MODEL_CONTEXT_CACHE: OnceLock<RwLock<HashMap<String, usize>>> = OnceLock::new();

fn model_cache() -> &'static RwLock<HashMap<String, usize>> {
    MODEL_CONTEXT_CACHE.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Known context window sizes for common local models.
/// Used as fallback when Ollama's `/api/show` endpoint is unreachable.
const KNOWN_CONTEXT_SIZES: &[(&str, usize)] = &[
    ("qwen2.5", 32768),
    ("qwen2", 32768),
    ("llama3", 8192),
    ("llama3.1", 131072),
    ("llama3.2", 131072),
    ("llama3.3", 131072),
    ("mistral", 8192),
    ("mixtral", 32768),
    ("deepseek", 65536),
    ("codellama", 16384),
    ("phi3", 4096),
    ("phi4", 16384),
    ("gemma2", 8192),
    ("nemotron", 4096),
    ("dolphin-mixtral", 32768),
];

/// Get the context window size for a given model name.
///
/// Checks the runtime cache first, then falls back to known model sizes,
/// and finally returns the default (8192) as a safe minimum.
pub fn get_model_context_size(model: &str) -> usize {
    let model_lower = model.to_lowercase();

    // Check cache
    if let Some(size) = model_cache()
        .read()
        .ok()
        .and_then(|c| c.get(&model_lower).copied())
    {
        return size;
    }

    // Check known models (longest prefix first to avoid "llama3" matching before "llama3.1")
    let mut sorted: Vec<_> = KNOWN_CONTEXT_SIZES.iter().collect();
    sorted.sort_by(|a, b| b.0.len().cmp(&a.0.len()));
    for (prefix, size) in sorted {
        if model_lower.contains(prefix) {
            return *size;
        }
    }

    // Default safe minimum
    8192
}

/// Query Ollama's `/api/show` endpoint to get a model's actual context window.
///
/// This is an async operation that contacts the local Ollama instance.
/// If it fails, we fall back to the known sizes or default.
pub async fn fetch_model_context_size(model: &str) -> Option<usize> {
    let model_lower = model.to_lowercase();

    // Already cached?
    if let Some(size) = model_cache()
        .read()
        .ok()
        .and_then(|c| c.get(&model_lower).copied())
    {
        return Some(size);
    }

    // Try to fetch from Ollama
    let url = format!("http://localhost:11434/api/show");
    let body = serde_json::json!({ "model": model, "verbose": true });

    match reqwest::Client::new()
        .post(&url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
    {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<serde_json::Value>().await {
                Ok(data) => {
                    // Ollama reports model info in model_info or modelfile
                    if let Some(ctx) = data
                        .get("model_info")
                        .and_then(|i| i.get("llama.context_length"))
                        .and_then(|v| v.as_u64())
                    {
                        let size = ctx as usize;
                        let _ = model_cache()
                            .write()
                            .map(|mut c| c.insert(model_lower, size));
                        return Some(size);
                    }
                    // Also check modelfile for num_ctx parameter
                    if let Some(ctx) =
                        data.get("modelfile")
                            .and_then(|m| m.as_str())
                            .and_then(|s| {
                                s.lines()
                                    .find(|l| l.trim().starts_with("num_ctx"))
                                    .and_then(|l| l.split_whitespace().nth(1))
                                    .and_then(|n| n.parse::<usize>().ok())
                            })
                    {
                        let _ = model_cache()
                            .write()
                            .map(|mut c| c.insert(model_lower, ctx));
                        return Some(ctx);
                    }
                    None
                }
                Err(_) => None,
            }
        }
        _ => None,
    }
}

/// Lightweight token-counting helpers.
pub fn count_tokens(text: &str) -> usize {
    if text.is_empty() {
        return 0;
    }
    (text.chars().count() + 3) / 4
}

pub fn count_messages_tokens(messages: &[crate::backend::Message]) -> usize {
    messages
        .iter()
        .map(|m| count_tokens(&m.content) + count_tokens(&m.role) + 8)
        .sum()
}

/// Returns true if the estimated token count of `text` exceeds a safe budget
/// for the given model. The safe budget is 75% of the model's context window
/// to leave room for the system prompt and tool definitions.
pub fn check_token_budget(text: &str, model: &str) -> bool {
    let context_size = get_model_context_size(model);
    let budget = (context_size as f64 * 0.75) as usize;
    count_tokens(text) > budget
}

/// Legacy signature: checks against an explicit limit.
/// Used where the model name is not available.
pub fn check_token_budget_against(text: &str, limit: usize) -> bool {
    count_tokens(text) > limit
}

/// Returns a human-readable warning message if the token count exceeds
/// the safe budget for the given model.
pub fn token_budget_warning(text: &str, model: &str) -> Option<String> {
    let context_size = get_model_context_size(model);
    let budget = (context_size as f64 * 0.75) as usize;
    let tokens = count_tokens(text);

    if tokens > budget {
        Some(format!(
            "⚠️ Estimated {} tokens exceeds the safe budget of {} (75% of {} context window). \
             The model may truncate your prompt.",
            tokens, budget, context_size
        ))
    } else if tokens > context_size / 2 {
        Some(format!(
            "⚠️ Estimated {} tokens. You are approaching the {} token context limit.",
            tokens, context_size
        ))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_empty() {
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn test_count_tokens_short() {
        assert_eq!(count_tokens("hello"), 2); // 5 chars / 4 = 1.25 -> 2
    }

    #[test]
    fn test_known_model_context_sizes() {
        assert_eq!(get_model_context_size("qwen2.5:7b-instruct"), 32768);
        assert_eq!(get_model_context_size("llama3.1:70b"), 131072);
        assert_eq!(get_model_context_size("mistral:v0.3"), 8192);
        assert_eq!(get_model_context_size("unknown-model:latest"), 8192);
    }

    #[test]
    fn test_budget_check_within_limit() {
        let text = "short text";
        let exceeded = check_token_budget(text, "qwen2.5:7b-instruct");
        // 32768 * 0.75 = 24576, text is ~3 tokens
        assert!(!exceeded);
    }

    #[test]
    fn test_token_budget_warning_triggers() {
        let text = "a".repeat(100000);
        let warning = token_budget_warning(&text, "qwen2.5:7b-instruct");
        assert!(warning.is_some());
        assert!(warning.unwrap().contains("exceeds"));
    }

    #[test]
    fn test_token_budget_warning_approaching() {
        // qwen2.5 context = 32768. 50% = 16384 tokens.
        // Need > 16384 * 4 = 65536 chars to be "approaching"
        let text = "a".repeat(70000);
        let warning = token_budget_warning(&text, "qwen2.5:7b-instruct");
        assert!(
            warning.is_some(),
            "Expected warning for text ~{} chars",
            text.len()
        );
    }

    #[test]
    fn test_no_warning_for_small_text() {
        let text = "hello world";
        let warning = token_budget_warning(&text, "qwen2.5:7b-instruct");
        assert!(warning.is_none());
    }

    #[test]
    fn test_check_token_budget_legacy() {
        assert!(check_token_budget_against("a".repeat(1000).as_str(), 100));
        assert!(!check_token_budget_against("short", 100));
    }
}
