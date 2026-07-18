use crate::engine::tokenizer::count_messages_tokens;
use rig::client::CompletionClient;
use rig::completion::Prompt;
use rig::providers::ollama;

const KEEP_RECENT_TURNS: usize = 4;

pub struct CompactResult {
    pub messages: Vec<crate::backend::Message>,
    pub compacted: usize,
}

async fn compaction_threshold(db: &crate::db::Database) -> usize {
    let raw = crate::backend::get_setting(db, "compaction_token_threshold").await
        .unwrap_or_else(|| "4096".to_string());
    raw.trim().parse::<usize>().unwrap_or(4096)
}

async fn compaction_model_name(db: &crate::db::Database) -> String {
    crate::backend::resolve_model_name(db, None, "compaction_model", "qwen2.5:7b-instruct").await
}

pub async fn maybe_compact(
    messages: Vec<crate::backend::Message>,
    ollama_client: &ollama::Client,
    db: &crate::db::Database,
) -> CompactResult {
    let threshold = compaction_threshold(db).await;
    let total_tokens = count_messages_tokens(&messages);
    if total_tokens <= threshold || messages.len() < KEEP_RECENT_TURNS + 2 {
        return CompactResult { messages, compacted: 0 };
    }

    let split_point = messages.len().saturating_sub(KEEP_RECENT_TURNS * 2);
    let (old, recent) = messages.split_at(split_point);

    let summary = summarize_turns(old, ollama_client, db).await;
    let mut compacted = Vec::with_capacity(recent.len() + 1);
    compacted.push(crate::backend::Message {
        id: 0,
        conversation_id: 0,
        role: "system".to_string(),
        content: format!("<memory>Previous conversation summary:\n{}</memory>", summary),
        created_at: chrono::Utc::now().naive_utc(),
        hide_evidence: false,
        attachments: vec![],
    });
    compacted.extend_from_slice(recent);

    CompactResult { messages: compacted, compacted: split_point }
}

async fn summarize_turns(
    old: &[crate::backend::Message],
    ollama_client: &ollama::Client,
    db: &crate::db::Database,
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

    let model = compaction_model_name(db).await;
    let agent = rig::agent::AgentBuilder::new(ollama_client.completion_model(&model))
        .preamble("You are a precise summarizer.")
        .build();
    match agent.prompt(&prompt).await {
        Ok(summary) => summary,
        Err(e) => {
            tracing::warn!(?e, "Memory compaction summarization failed, using truncated raw text");
            text.chars().take(500).collect()
        }
    }
}
