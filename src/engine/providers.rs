//! LLM provider abstraction.
//!
//! Overseer was Ollama-only, which made replies (and tool-calling) unreliable.
//! This module adds cloud providers (OpenAI / Anthropic) via direct HTTP so the
//! app can use a fast, reliable model when configured, while keeping local
//! Ollama as the default/offline option. Provider + keys live in settings.
//!
//! Foundation: `complete_chat` (system + user → assistant text) is wired into the
//! direct-chat path. Native function-calling and the full dispatch swap build on
//! this next.

use crate::db::Database;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Ollama,
    OpenAI,
    Anthropic,
}

impl Provider {
    pub fn parse(s: &str) -> Provider {
        match s.trim().to_lowercase().as_str() {
            "openai" => Provider::OpenAI,
            "anthropic" | "claude" => Provider::Anthropic,
            _ => Provider::Ollama,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ProviderConfig {
    pub provider: Provider,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl ProviderConfig {
    /// A cloud provider is usable only if a provider is selected AND a key is set.
    pub fn is_cloud_ready(&self) -> bool {
        !matches!(self.provider, Provider::Ollama) && !self.api_key.trim().is_empty()
    }
}

/// Load the configured provider from settings.
pub async fn load_config(db: &Database) -> ProviderConfig {
    let provider = Provider::parse(&crate::backend::get_setting_or(db, "llm_provider", "ollama").await);
    match provider {
        Provider::OpenAI => ProviderConfig {
            provider,
            api_key: crate::backend::get_setting(db, "openai_api_key").await,
            base_url: {
                let b = crate::backend::get_setting_or(db, "openai_base_url", "https://api.openai.com/v1").await;
                b.trim_end_matches('/').to_string()
            },
            model: crate::backend::get_setting_or(db, "openai_model", "gpt-4o-mini").await,
        },
        Provider::Anthropic => ProviderConfig {
            provider,
            api_key: crate::backend::get_setting(db, "anthropic_api_key").await,
            base_url: {
                let b = crate::backend::get_setting_or(db, "anthropic_base_url", "https://api.anthropic.com").await;
                b.trim_end_matches('/').to_string()
            },
            model: crate::backend::get_setting_or(db, "anthropic_model", "claude-3-5-sonnet-latest").await,
        },
        Provider::Ollama => ProviderConfig {
            provider,
            api_key: String::new(),
            base_url: "http://localhost:11434".to_string(),
            model: crate::backend::get_setting_or(db, "default_chat_model", "qwen2.5:7b-instruct").await,
        },
    }
}

/// Single-turn chat completion against the configured cloud provider.
/// Returns the assistant text. Only valid for OpenAI/Anthropic (see
/// `is_cloud_ready`); the Ollama path stays on the existing rig client.
pub async fn complete_chat(cfg: &ProviderConfig, system: &str, user: &str) -> anyhow::Result<String> {
    let client = reqwest::Client::new();
    match cfg.provider {
        Provider::OpenAI => {
            let body = serde_json::json!({
                "model": cfg.model,
                "messages": [
                    { "role": "system", "content": system },
                    { "role": "user", "content": user },
                ],
            });
            let resp = client
                .post(format!("{}/chat/completions", cfg.base_url))
                .bearer_auth(&cfg.api_key)
                .json(&body)
                .send()
                .await?;
            let status = resp.status();
            let v: serde_json::Value = resp.json().await?;
            if !status.is_success() {
                anyhow::bail!("OpenAI error {}: {}", status, v);
            }
            Ok(v["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or_default()
                .to_string())
        }
        Provider::Anthropic => {
            let body = serde_json::json!({
                "model": cfg.model,
                "max_tokens": 1024,
                "system": system,
                "messages": [ { "role": "user", "content": user } ],
            });
            let resp = client
                .post(format!("{}/v1/messages", cfg.base_url))
                .header("x-api-key", &cfg.api_key)
                .header("anthropic-version", "2023-06-01")
                .json(&body)
                .send()
                .await?;
            let status = resp.status();
            let v: serde_json::Value = resp.json().await?;
            if !status.is_success() {
                anyhow::bail!("Anthropic error {}: {}", status, v);
            }
            Ok(v["content"][0]["text"].as_str().unwrap_or_default().to_string())
        }
        Provider::Ollama => {
            anyhow::bail!("complete_chat is for cloud providers; use the Ollama path")
        }
    }
}

// ---------------------------------------------------------------------------
// Native function-calling
// ---------------------------------------------------------------------------

/// A tool the model may call, described with a JSON-Schema for its arguments.
#[derive(Debug, Clone)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON-Schema object describing the tool's arguments.
    pub parameters: serde_json::Value,
}

/// A tool call the model wants to make.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// One turn of the plan→act→observe conversation.
#[derive(Debug, Clone)]
pub enum AgentMessage {
    User(String),
    Assistant {
        text: Option<String>,
        tool_calls: Vec<ToolCall>,
    },
    /// Result of executing a tool call, fed back as an observation.
    ToolResult {
        id: String,
        name: String,
        content: String,
    },
}

/// The model's response for one step: either a final answer (`text`, no calls)
/// or a set of tool calls to execute and feed back.
#[derive(Debug, Clone, Default)]
pub struct AgentTurn {
    pub text: Option<String>,
    pub tool_calls: Vec<ToolCall>,
}

/// One agentic step against a cloud provider with tools available.
/// Returns the model's tool calls (to execute + observe) or a final answer.
pub async fn complete_with_tools(
    cfg: &ProviderConfig,
    system: &str,
    messages: &[AgentMessage],
    tools: &[ToolSpec],
) -> anyhow::Result<AgentTurn> {
    let client = reqwest::Client::new();
    match cfg.provider {
        Provider::OpenAI => openai_with_tools(&client, cfg, system, messages, tools).await,
        Provider::Anthropic => anthropic_with_tools(&client, cfg, system, messages, tools).await,
        Provider::Ollama => {
            anyhow::bail!("complete_with_tools is for cloud providers")
        }
    }
}

async fn openai_with_tools(
    client: &reqwest::Client,
    cfg: &ProviderConfig,
    system: &str,
    messages: &[AgentMessage],
    tools: &[ToolSpec],
) -> anyhow::Result<AgentTurn> {
    let mut wire: Vec<serde_json::Value> = vec![serde_json::json!({
        "role": "system", "content": system,
    })];
    for m in messages {
        match m {
            AgentMessage::User(s) => wire.push(serde_json::json!({ "role": "user", "content": s })),
            AgentMessage::Assistant { text, tool_calls } => {
                let calls: Vec<serde_json::Value> = tool_calls
                    .iter()
                    .map(|c| {
                        serde_json::json!({
                            "id": c.id,
                            "type": "function",
                            "function": { "name": c.name, "arguments": c.arguments.to_string() },
                        })
                    })
                    .collect();
                let mut msg = serde_json::json!({ "role": "assistant" });
                msg["content"] = text.clone().map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
                if !calls.is_empty() {
                    msg["tool_calls"] = serde_json::Value::Array(calls);
                }
                wire.push(msg);
            }
            AgentMessage::ToolResult { id, content, .. } => wire.push(serde_json::json!({
                "role": "tool", "tool_call_id": id, "content": content,
            })),
        }
    }

    let tool_defs: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "type": "function",
                "function": { "name": t.name, "description": t.description, "parameters": t.parameters },
            })
        })
        .collect();

    let mut body = serde_json::json!({ "model": cfg.model, "messages": wire });
    if !tool_defs.is_empty() {
        body["tools"] = serde_json::Value::Array(tool_defs);
    }

    let resp = client
        .post(format!("{}/chat/completions", cfg.base_url))
        .bearer_auth(&cfg.api_key)
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    let v: serde_json::Value = resp.json().await?;
    if !status.is_success() {
        anyhow::bail!("OpenAI error {}: {}", status, v);
    }

    let msg = &v["choices"][0]["message"];
    let mut turn = AgentTurn::default();
    if let Some(text) = msg["content"].as_str() {
        if !text.is_empty() {
            turn.text = Some(text.to_string());
        }
    }
    if let Some(calls) = msg["tool_calls"].as_array() {
        for c in calls {
            let name = c["function"]["name"].as_str().unwrap_or_default().to_string();
            let args_str = c["function"]["arguments"].as_str().unwrap_or("{}");
            let arguments = serde_json::from_str(args_str).unwrap_or(serde_json::Value::Object(Default::default()));
            turn.tool_calls.push(ToolCall {
                id: c["id"].as_str().unwrap_or_default().to_string(),
                name,
                arguments,
            });
        }
    }
    Ok(turn)
}

async fn anthropic_with_tools(
    client: &reqwest::Client,
    cfg: &ProviderConfig,
    system: &str,
    messages: &[AgentMessage],
    tools: &[ToolSpec],
) -> anyhow::Result<AgentTurn> {
    // Anthropic requires tool_result blocks to live in a user message following
    // the assistant tool_use; merge consecutive tool results into one user turn.
    let mut wire: Vec<serde_json::Value> = Vec::new();
    for m in messages {
        match m {
            AgentMessage::User(s) => wire.push(serde_json::json!({
                "role": "user", "content": [{ "type": "text", "text": s }],
            })),
            AgentMessage::Assistant { text, tool_calls } => {
                let mut content: Vec<serde_json::Value> = Vec::new();
                if let Some(t) = text {
                    if !t.is_empty() {
                        content.push(serde_json::json!({ "type": "text", "text": t }));
                    }
                }
                for c in tool_calls {
                    content.push(serde_json::json!({
                        "type": "tool_use", "id": c.id, "name": c.name, "input": c.arguments,
                    }));
                }
                wire.push(serde_json::json!({ "role": "assistant", "content": content }));
            }
            AgentMessage::ToolResult { id, content, .. } => {
                let block = serde_json::json!({
                    "type": "tool_result", "tool_use_id": id, "content": content,
                });
                // Append to a trailing user message if the previous entry is one.
                if let Some(last) = wire.last_mut() {
                    if last["role"] == "user" {
                        if let Some(arr) = last["content"].as_array_mut() {
                            arr.push(block);
                            continue;
                        }
                    }
                }
                wire.push(serde_json::json!({ "role": "user", "content": [block] }));
            }
        }
    }

    let tool_defs: Vec<serde_json::Value> = tools
        .iter()
        .map(|t| {
            serde_json::json!({
                "name": t.name, "description": t.description, "input_schema": t.parameters,
            })
        })
        .collect();

    let mut body = serde_json::json!({
        "model": cfg.model,
        "max_tokens": 4096,
        "system": system,
        "messages": wire,
    });
    if !tool_defs.is_empty() {
        body["tools"] = serde_json::Value::Array(tool_defs);
    }

    let resp = client
        .post(format!("{}/v1/messages", cfg.base_url))
        .header("x-api-key", &cfg.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await?;
    let status = resp.status();
    let v: serde_json::Value = resp.json().await?;
    if !status.is_success() {
        anyhow::bail!("Anthropic error {}: {}", status, v);
    }

    let mut turn = AgentTurn::default();
    let mut text = String::new();
    if let Some(blocks) = v["content"].as_array() {
        for b in blocks {
            match b["type"].as_str() {
                Some("text") => text.push_str(b["text"].as_str().unwrap_or_default()),
                Some("tool_use") => turn.tool_calls.push(ToolCall {
                    id: b["id"].as_str().unwrap_or_default().to_string(),
                    name: b["name"].as_str().unwrap_or_default().to_string(),
                    arguments: b["input"].clone(),
                }),
                _ => {}
            }
        }
    }
    if !text.trim().is_empty() {
        turn.text = Some(text);
    }
    Ok(turn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_parse() {
        assert_eq!(Provider::parse("OpenAI"), Provider::OpenAI);
        assert_eq!(Provider::parse("anthropic"), Provider::Anthropic);
        assert_eq!(Provider::parse("claude"), Provider::Anthropic);
        assert_eq!(Provider::parse("ollama"), Provider::Ollama);
        assert_eq!(Provider::parse("whatever"), Provider::Ollama);
    }

    #[test]
    fn cloud_ready_requires_key() {
        let mut c = ProviderConfig {
            provider: Provider::OpenAI,
            api_key: String::new(),
            base_url: "x".into(),
            model: "gpt-4o-mini".into(),
        };
        assert!(!c.is_cloud_ready(), "no key -> not ready");
        c.api_key = "sk-test".into();
        assert!(c.is_cloud_ready());
        c.provider = Provider::Ollama;
        assert!(!c.is_cloud_ready(), "ollama is never 'cloud ready'");
    }

    #[tokio::test]
    async fn config_defaults_to_ollama() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let cfg = load_config(&db).await;
        assert_eq!(cfg.provider, Provider::Ollama);
        assert!(!cfg.is_cloud_ready());
    }
}
