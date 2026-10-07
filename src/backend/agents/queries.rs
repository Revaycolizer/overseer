use crate::db::Database;
use crate::engine::utils::discovery::AgentInfo;
use std::collections::HashMap;

pub async fn get_agents(db: &Database) -> anyhow::Result<Vec<AgentInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code, response_format FROM agents"
    )
    .fetch_all(&db.pool)
    .await?;

    let tool_rows = sqlx::query(
        "SELECT at.agent_id, t.name
         FROM agent_tools at
         JOIN tools t ON t.id = at.tool_id",
    )
    .fetch_all(&db.pool)
    .await?;

    let mut tools_by_agent: HashMap<i64, Vec<String>> = HashMap::new();
    for row in tool_rows {
        let agent_id: i64 = row.get("agent_id");
        let tool_name: String = row.get("name");
        tools_by_agent.entry(agent_id).or_default().push(tool_name);
    }

    let mut agents = Vec::new();
    for r in rows {
        let agent_id: i64 = r.get("id");

        let domain: String = r.get::<Option<String>, _>("domain").unwrap_or_default();

        agents.push(AgentInfo {
            id: agent_id,
            name: r.get("name"),
            description: r
                .get::<Option<String>, _>("description")
                .unwrap_or_default(),
            instructions: r
                .get::<Option<String>, _>("instructions")
                .unwrap_or_default(),
            model: r.get::<Option<String>, _>("model").unwrap_or_default(),
            domain: domain.clone(),
            version: r.get::<Option<String>, _>("version").unwrap_or_default(),
            code: r.get("code"),
            tools: tools_by_agent.remove(&agent_id).unwrap_or_default(),
            source_files: vec![],
            response_format: r
                .get::<Option<String>, _>("response_format")
                .unwrap_or_else(|| "json".to_string()),
        });
    }

    Ok(agents)
}

pub async fn get_all_agents(db: &Database) -> anyhow::Result<Vec<AgentInfo>> {
    get_agents(db).await
}

pub async fn get_agent_by_domain(db: &Database, domain: &str) -> anyhow::Result<Option<AgentInfo>> {
    let agents = get_agents(db).await?;
    let target = domain.trim().to_lowercase();
    for agent in agents {
        if agent.domain.trim().to_lowercase() == target {
            return Ok(Some(agent));
        }
    }
    Ok(None)
}
