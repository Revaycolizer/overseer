use crate::db::Database;
use crate::engine::orchestrator::planning::{master::MasterPlanner, decisions::*, planner_utils, planner::MacroTask};
use crate::engine::orchestrator::discovery;
use crate::engine::utils::discovery::AgentInfo;
use rig::providers::ollama;
use rig::completion::Message;
use std::time::Duration;
use tokio::time::timeout;

pub async fn resolve_macro_tasks(
    db: &Database,
    master_planner: &MasterPlanner,
    _all_agents: &[AgentInfo],
    clean_query: &str,
    history: Vec<Message>,
    decision: &OrchestrationDecision,
    ollama_client: &ollama::Client,
) -> Vec<MacroTask> {
    if matches!(decision.route, OrchestrationRoute::SingleDomain) {
        let synthetic = serde_json::json!([{"domain": decision.domain.clone().unwrap_or_else(|| "chat".to_string()), "task": clean_query}]).to_string();
        return planner_utils::sanitize_plan(&synthetic, clean_query);
    }

    let http_client = reqwest::Client::new();
    let discovered_agents = discovery::discover_semantic_agents(db, &http_client, clean_query, 8, ollama_client).await;
    
    let mut macro_tasks = match timeout(Duration::from_millis(90000), master_planner.plan(ollama_client, clean_query, history.clone(), discovered_agents.clone())).await {
        Ok(plan) => planner_utils::sanitize_plan(&plan, clean_query),
        Err(_) => vec![],
    };

    // ARCHITECTURE V2.7: DECOMPOSITION INTEGRITY CHECK
    let lower_query = clean_query.to_lowercase();
    let artifact_keywords = ["pdf", "docx", "xlsx", "csv", "report"];
    let is_artifact_task = artifact_keywords.iter().any(|&kw| lower_query.contains(kw));

    if is_artifact_task && macro_tasks.len() == 1 {
        let retry_prompt = format!("{}\n\n⚠️ COORDINATION FAILURE: Decompose this into at least TWO steps: 1. Research 2. Generation.", clean_query);
        if let Ok(plan) = timeout(Duration::from_millis(60000), master_planner.plan(ollama_client, &retry_prompt, history, discovered_agents)).await {
            macro_tasks = planner_utils::sanitize_plan(&plan, clean_query);
        }
        if macro_tasks.len() == 1 { macro_tasks = planner_utils::force_decompose_artifact_plan(clean_query); }
    }
    macro_tasks
}
