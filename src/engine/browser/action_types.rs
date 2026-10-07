use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub max_attempts: u8,
    pub backoff_ms: u64,
    pub max_backoff_ms: u64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff_ms: 500,
            max_backoff_ms: 5000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionPolicy {
    pub allowed_domains: Vec<String>,
    pub max_actions: u32,
    pub max_depth: u32,
    pub action_timeout_ms: u64,
    pub session_timeout_ms: u64,
}

impl Default for ActionPolicy {
    fn default() -> Self {
        Self {
            allowed_domains: vec![],
            max_actions: 500,
            max_depth: 10,
            action_timeout_ms: 30000,
            session_timeout_ms: 3600000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    pub success: bool,
    pub url: String,
    pub title: String,
    pub dom_changed: bool,
    pub navigation_occurred: bool,
    pub retries_used: u8,
    pub duration_ms: u64,
}
