use crate::engine::tokenizer::count_messages_tokens;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::ollama;
use serde::Deserialize;

const KEEP_RECENT_TURNS: usize = 4;
const DEFAULT_NUM_CTX: usize = 8192;

#[derive(Deserialize)]
struct OllamaShowResponse {
    #[serde(default)]
    parameters: OllamaParameters,
    #[serde(default)]
    model_info: serde_json::Value,
}

#[derive(Deserialize, Default)]
struct OllamaParameters {
    #[serde(rename = "num_ctx", default)]
    num_ctx: Option<usize>,
}

pub struct CompactResult {
    pub messages: Vec<crate::backend::Message>,
    pub compacted: usize,
}

async fn context_ratio(db: &crate::db::Database) -> f64 {
    let raw = crate::backend::get_setting(db, "compaction_context_ratio").await;
    if raw.is_empty() {
        return 0.75;
    }
    raw.trim().parse::<f64>().unwrap_or(0.75)
}

async fn compaction_model_name(db: &crate::db::Database) -> String {
    crate::backend::resolve_model_name(db, None, "compaction_model", "qwen2.5:7b-instruct").await
}

async fn fetch_num_ctx(model: &str) -> Option<usize> {
    let url = format!("http://localhost:11434/api/show");
    let body = serde_json::json!({ "model": model });
    let resp = reqwest::Client::new()
        .post(&url)
        .json(&body)
        .send()
        .await
        .ok()?;
    let show: OllamaShowResponse = resp.json().await.ok()?;

    if let Some(n) = show.parameters.num_ctx {
        return Some(n);
    }

    // Fall back to model_info.*.context_length
    if let Some(info) = show.model_info.as_object() {
        for (key, val) in info {
            if key.ends_with("context_length") {
                if let Some(n) = val.as_u64() {
                    return Some(n as usize);
                }
            }
        }
    }

    None
}

pub async fn maybe_compact(
    messages: Vec<crate::backend::Message>,
    ollama_client: &ollama::Client,
    db: &crate::db::Database,
) -> CompactResult {
    let model = compaction_model_name(db).await;
    let ratio = context_ratio(db).await;
    let num_ctx = fetch_num_ctx(&model).await.unwrap_or(DEFAULT_NUM_CTX);
    let threshold = (num_ctx as f64 * ratio) as usize;

    let total_tokens = count_messages_tokens(&messages);
    if total_tokens <= threshold || messages.len() < KEEP_RECENT_TURNS + 2 {
        return CompactResult {
            messages,
            compacted: 0,
        };
    }

    let split_point = messages.len().saturating_sub(KEEP_RECENT_TURNS * 2);
    let (old, recent) = messages.split_at(split_point);

    let summary = summarize_turns(old, ollama_client, &model).await;
    let mut compacted = Vec::with_capacity(recent.len() + 1);
    compacted.push(crate::backend::Message {
        id: 0,
        conversation_id: 0,
        role: "system".to_string(),
        content: format!(
            "<memory>Previous conversation summary:\n{}</memory>",
            summary
        ),
        created_at: chrono::Utc::now().naive_utc(),
        hide_evidence: false,
        attachments: vec![],
    });
    compacted.extend_from_slice(recent);

    CompactResult {
        messages: compacted,
        compacted: split_point,
    }
}

async fn summarize_turns(
    old: &[crate::backend::Message],
    ollama_client: &ollama::Client,
    model: &str,
) -> String {
    let text: String = old
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(4000)
        .collect();

    let prompt = format!(
        "Summarize the following conversation in 200 words or fewer. \
         Capture key facts, decisions, and context.\n\n{}",
        text
    );

    let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model(model))
        .preamble("You are a precise summarizer.")
        .build();
    match agent.prompt(&prompt).await {
        Ok(summary) => summary,
        Err(e) => {
            tracing::warn!(
                ?e,
                "Memory compaction summarization failed, using truncated raw text"
            );
            text.chars().take(500).collect()
        }
    }
}
