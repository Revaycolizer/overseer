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
