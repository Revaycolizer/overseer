use serde::{Deserialize, Serialize};
use crate::engine::orchestrator::native::native_checks::*;
use crate::engine::orchestrator::discovery::discovery;
use crate::engine::utils::discovery::AgentInfo;
use rig::agent::AgentBuilder;
use rig::completion::{Message, Prompt, PromptError};
use rig::providers::ollama;
use rig::prelude::CompletionClient;

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
        reason: "Heuristic routing.".to_string(),
    }
}

pub async fn decide_orchestration_with_llm(
    ollama_client: &ollama::Client,
    query: &str,
    history: &[Message],
    memory_context: &str,
) -> Result<OrchestrationDecision, String> {
    let model = "qwen2.5:7b-instruct"; // Default router model
    let agent = AgentBuilder::new(ollama_client.completion_model(model))
        .preamble("You are the Overseer Router. Your job is to categorize a user request into one of three orchestration routes based on its complexity and context.\n\n\
                  ROUTES:\n\
                  1. direct_chat: Simple greetings, general conversation, or follow-ups that don't require external tools or specific domains.\n\
                  2. single_domain: Tasks that can be completed immediately by a single specialized agent (e.g., 'What time is it?', 'Take a screenshot', 'Find my resume').\n\
                  3. multi_step: Complex tasks requiring information gathering (research) before an action (generation/execution). Example: 'Generate a PDF about malaria' requires researching malaria first. Any task where the 'how' or 'what' is not currently in the context should be multi_step.\n\n\
                  CONTEXT:\n\
                  Review the RELEVANT_CONTEXT. If the information needed to perform the task is NOT in the context, choose 'multi_step' even if it sounds like a single-domain task.\n\n\
                  OUTPUT:\n\
                  Respond with a single JSON object: {\"route\": \"direct_chat|single_domain|multi_step\", \"domain\": \"domain_name_or_null\", \"personality_mode\": \"assistant|operator|orchestrator\", \"reason\": \"brief explanation\"}")
        .build();

    let history_str = history.iter().map(|m| format!("{:?}", m)).collect::<Vec<_>>().join("\n");
    let prompt = format!(
        "USER_QUERY: {}\n\n\
         RELEVANT_CONTEXT: {}\n\n\
         CONVERSATION_HISTORY:\n{}\n\n\
         Decide the route.",
        query, memory_context, history_str
    );

    let response = agent.prompt(&prompt).await.map_err(|e: PromptError| e.to_string())?;
    let trimmed = response.trim();
    
    // Extract JSON
    let json_str = if let Some(start) = trimmed.find('{') {
        if let Some(end) = trimmed.rfind('}') {
            &trimmed[start..=end]
        } else { trimmed }
    } else { trimmed };

    serde_json::from_str::<OrchestrationDecision>(json_str).map_err(|e| format!("Failed to parse router JSON: {}", e))
}

pub async fn decide_orchestration(
    db: &crate::db::Database,
    _all_agents: &[AgentInfo],
    query: &str,
    history: &[Message],
    memory_context: &str,
    ollama_client: &ollama::Client,
    precomputed_fast_path_domain: Option<&str>,
    has_artifacts: bool,
) -> Result<OrchestrationDecision, String> {
    // 1. FAST-PATH (Regex): Always bypass LLM for extremely obvious commands.
    if let Some(domain) = precomputed_fast_path_domain {
        return Ok(OrchestrationDecision { route: OrchestrationRoute::SingleDomain, domain: Some(domain.to_string()), personality_mode: "operator".to_string(), reason: format!("Fast-path routing match for domain: {}", domain) });
    }
    
    // 2. LLM-ROUTING (Smarter Case): Use the model to decide complexity.
    match tokio::time::timeout(std::time::Duration::from_millis(5000), decide_orchestration_with_llm(ollama_client, query, history, memory_context)).await {
        Ok(Ok(decision)) => return Ok(decision),
        Ok(Err(e)) => eprintln!("LLM Router error: {}. Falling back to heuristic.", e),
        Err(_) => eprintln!("LLM Router timed out. Falling back to heuristic."),
    }

    // 3. HEURISTIC FALLBACK: If LLM is slow or fails.
    Ok(heuristic_decision(db, query, precomputed_fast_path_domain, has_artifacts).await)
}
