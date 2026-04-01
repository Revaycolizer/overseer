use serde::{Deserialize, Serialize};
use crate::engine::orchestrator::native::native_checks::*;
use crate::engine::orchestrator::discovery::discovery;
use crate::engine::utils::discovery::AgentInfo;
use rig::completion::Message;
use rig::providers::ollama;

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

pub fn should_bypass_final_synthesizer(domains: &[String]) -> bool {
    !domains.is_empty() && domains.iter().all(|d| matches!(d.as_str(), "computer" | "desktop"))
}

pub async fn heuristic_decision(
    db: &crate::db::Database,
    query: &str,
    precomputed_fast_path_domain: Option<&str>,
    has_artifacts: bool,
) -> OrchestrationDecision {
    if discovery::looks_like_planner_request(query) {
        return OrchestrationDecision {
            route: OrchestrationRoute::MultiStep,
            domain: None,
            personality_mode: "orchestrator".to_string(),
            reason: "Workflow/planning request should go through multi-step orchestration.".to_string(),
        };
    }

    let inferred_domain = if let Some(domain) = precomputed_fast_path_domain { Some(domain.to_string()) } else { discovery::discover_fast_path_domain(db, query, has_artifacts).await };

    if inferred_domain.is_none() && (query.len() < 40 || ["hi", "hello", "hey", "who are you"].iter().any(|&g| query.to_lowercase().starts_with(g))) {
        return OrchestrationDecision { route: OrchestrationRoute::DirectChat, domain: Some("chat".to_string()), personality_mode: "assistant".to_string(), reason: "Conversation query routed to direct chat.".to_string() };
    }

    OrchestrationDecision {
        route: if inferred_domain.is_some() { OrchestrationRoute::SingleDomain } else { OrchestrationRoute::MultiStep },
        domain: inferred_domain,
        personality_mode: "operator".to_string(),
        reason: "Generic heuristic routing.".to_string(),
    }
}

pub async fn decide_orchestration(
    db: &crate::db::Database,
    all_agents: &[AgentInfo],
    query: &str,
    history: &[Message],
    memory_context: &str,
    ollama_client: &ollama::Client,
    precomputed_fast_path_domain: Option<&str>,
    has_artifacts: bool,
) -> Result<OrchestrationDecision, String> {
    if let Some(domain) = precomputed_fast_path_domain {
        return Ok(OrchestrationDecision { route: OrchestrationRoute::SingleDomain, domain: Some(domain.to_string()), personality_mode: "operator".to_string(), reason: format!("Fast-path routing match for domain: {}", domain) });
    }
    if is_affirmative_follow_up(query) { /* Intent recognition logic... */ }
    Ok(heuristic_decision(db, query, precomputed_fast_path_domain, has_artifacts).await)
}
