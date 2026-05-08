use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrchestrationRoute { DirectChat, SingleDomain, MultiStep }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestrationDecision {
    pub route: OrchestrationRoute,
    pub domain: Option<String>,
    pub personality_mode: String,
    pub reason: String,
}
