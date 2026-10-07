pub mod llm;
pub mod types;

use crate::engine::orchestrator::discovery;
use rig::completion::Message;
use rig::providers::ollama;
pub use types::*;

pub fn should_bypass_final_synthesizer(domains: &[String]) -> bool {
    !domains.is_empty()
        && domains
            .iter()
            .all(|d| matches!(d.as_str(), "computer" | "desktop"))
}

pub async fn heuristic_decision(
    db: &crate::db::Database,
    query: &str,
    fast_path: Option<&str>,
    has_artifacts: bool,
) -> OrchestrationDecision {
    if discovery::looks_like_planner_request(query) {
        return OrchestrationDecision {
            route: OrchestrationRoute::MultiStep,
            domain: None,
            personality_mode: "orchestrator".to_string(),
            reason: "Workflow request.".to_string(),
        };
    }

    let lq = query.to_lowercase();
    if [
        "pdf",
        "docx",
        "xlsx",
        "csv",
        "report",
        "research",
        "summarize",
    ]
    .iter()
    .any(|&kw| lq.contains(kw))
    {
        return OrchestrationDecision {
            route: OrchestrationRoute::MultiStep,
            domain: None,
            personality_mode: "orchestrator".to_string(),
            reason: "Complex intent detected.".to_string(),
        };
    }

    let inferred = if let Some(d) = fast_path {
        Some(d.to_string())
    } else {
        discovery::discover_fast_path_domain(db, query, has_artifacts).await
    };
    if inferred.is_none()
        && (query.len() < 40 || ["hi", "hello", "hey"].iter().any(|&g| lq.starts_with(g)))
    {
        return OrchestrationDecision {
            route: OrchestrationRoute::DirectChat,
            domain: Some("chat".to_string()),
            personality_mode: "assistant".to_string(),
            reason: "Conversation.".to_string(),
        };
    }

    OrchestrationDecision {
        route: if inferred.is_some() {
            OrchestrationRoute::SingleDomain
        } else {
            OrchestrationRoute::MultiStep
        },
        domain: inferred,
        personality_mode: "operator".to_string(),
        reason: "Heuristic.".to_string(),
    }
}

pub async fn decide_orchestration(
    db: &crate::db::Database,
    _all_agents: &[crate::engine::utils::discovery::AgentInfo],
    query: &str,
    history: &[Message],
    memory_context: &str,
    ollama_client: &ollama::Client,
    fast_path: Option<&str>,
    has_artifacts: bool,
) -> Result<OrchestrationDecision, String> {
    let lq = query.to_lowercase();
    if [
        "pdf",
        "docx",
        "xlsx",
        "csv",
        "report",
        "research",
        "summarize",
    ]
    .iter()
    .any(|&kw| lq.contains(kw))
    {
        return Ok(OrchestrationDecision {
            route: OrchestrationRoute::MultiStep,
            domain: None,
            personality_mode: "orchestrator".to_string(),
            reason: "Heuristic override.".to_string(),
        });
    }

    if let Some(domain) = fast_path {
        return Ok(OrchestrationDecision {
            route: OrchestrationRoute::SingleDomain,
            domain: Some(domain.to_string()),
            personality_mode: "operator".to_string(),
            reason: "Fast-path.".to_string(),
        });
    }

    let model =
        crate::backend::resolve_model_name(db, None, "router_model", "qwen2.5:7b-instruct").await;

    match tokio::time::timeout(
        std::time::Duration::from_millis(5000),
        llm::decide_orchestration_with_llm(ollama_client, &model, query, history, memory_context),
    )
    .await
    {
        Ok(Ok(decision)) => Ok(decision),
        _ => Ok(heuristic_decision(db, query, fast_path, has_artifacts).await),
    }
}
