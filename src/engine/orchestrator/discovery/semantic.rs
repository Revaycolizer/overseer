// src/engine/orchestrator/discovery/semantic.rs
use crate::db::Database;
use crate::engine::utils::discovery::AgentInfo;
use rig::providers::ollama;

pub async fn discover_semantic_agents(
    db: &Database,
    _client: &reqwest::Client,
    query: &str,
    limit: usize,
    ollama_client: &ollama::Client,
) -> Vec<AgentInfo> {
    let mut supervisors = sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT * FROM supervisors ORDER BY routing_priority DESC"
    ).fetch_all(&db.pool).await.unwrap_or_default();

    let mut agents = Vec::new();
    let mut added_domains = std::collections::HashSet::new();
    let core_domains = vec!["stealth", "office", "web", "chat"];

    // Ensure core domains are present
    for domain in core_domains {
        if let Ok(Some(s)) = sqlx::query_as::<_, crate::db::Supervisor>(
            "SELECT * FROM supervisors WHERE domain = ?"
        ).bind(domain).fetch_optional(&db.pool).await {
            if added_domains.insert(s.domain.clone()) {
                agents.push(map_supervisor_to_agent(db, s).await);
            }
        }
    }

    for s in supervisors {
        if added_domains.insert(s.domain.clone()) {
            agents.push(map_supervisor_to_agent(db, s).await);
        }
    }

    super::rerank::rerank_agents(query, agents, limit, ollama_client).await
}

async fn map_supervisor_to_agent(db: &Database, s: crate::db::Supervisor) -> AgentInfo {
    let tools = crate::backend::get_supervisor_tools(db, s.id).await.unwrap_or_default();
    AgentInfo {
        id: s.id, name: s.name, description: s.description.unwrap_or_default(),
        tools: tools.into_iter().map(|t| t.name).collect(),
        instructions: s.instructions.unwrap_or_default(),
        model: s.model.unwrap_or_default(), domain: s.domain,
        version: s.version.unwrap_or_default(), code: s.code,
        source_files: vec![], response_format: s.response_format,
    }
}
