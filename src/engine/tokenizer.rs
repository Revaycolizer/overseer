/// Lightweight token-counting helpers.
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