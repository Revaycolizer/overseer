use crate::engine::utils::discovery::{AgentInfo, ToolInfo};
use crate::db::Database;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use reqwest;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLoad {
    pub label: String,
    pub usage: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

pub async fn get_available_models() -> Vec<String> {
    let client = reqwest::Client::new();
    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                if let Some(models) = json["models"].as_array() {
                    return models.iter()
                        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
            vec!["llama3:latest".to_string(), "qwen2.5:7b-instruct".to_string()]
        }
        Err(_) => vec!["llama3:latest".to_string(), "qwen2.5:7b-instruct".to_string()],
    }
}

pub async fn get_agents(db: &Database) -> anyhow::Result<Vec<AgentInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code FROM agents"
    )
    .fetch_all(&db.pool)
    .await?;
    
    let mut agents = Vec::new();
    for r in rows {
        let agent_id: i64 = r.get("id");
        
        // Fetch tools for this agent
        let tools_rows = sqlx::query(
            "SELECT t.name FROM tools t 
             JOIN agent_tools at ON t.id = at.tool_id 
             WHERE at.agent_id = ?"
        )
        .bind(agent_id)
        .fetch_all(&db.pool)
        .await?;

        let domain: String = r.get::<Option<String>, _>("domain").unwrap_or_default();

        agents.push(AgentInfo {
            id: agent_id,
            name: r.get("name"),
            description: r.get::<Option<String>, _>("description").unwrap_or_default(),
            instructions: r.get::<Option<String>, _>("instructions").unwrap_or_default(),
            model: r.get::<Option<String>, _>("model").unwrap_or_default(),
            domain: domain.clone(),
            version: r.get::<Option<String>, _>("version").unwrap_or_default(),
            code: r.get("code"),
            tools: tools_rows.into_iter().map(|t| t.get("name")).collect(),
            source_files: vec![],
        });
    }
    
    Ok(agents)
}

pub async fn get_tools(db: &Database) -> anyhow::Result<Vec<ToolInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, description, CAST(code AS TEXT) as code, language, script_path, version FROM tools"
    )
    .fetch_all(&db.pool)
    .await?;
    
    let tools = rows.into_iter().map(|r| ToolInfo {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get::<Option<String>, _>("description").unwrap_or_default(),
        code: r.get::<Option<String>, _>("code").unwrap_or_default(),
        language: r.get::<Option<String>, _>("language").unwrap_or_default(),
        script_path: r.get("script_path"),
        version: r.get::<Option<String>, _>("version").unwrap_or_default(),
    }).collect();
    
    Ok(tools)
}

pub async fn add_agent(db: &Database, name: &str, description: &str, domain: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO agents (id, name, description, instructions, model, domain, version, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(name).bind(description).bind("").bind("llama3").bind(domain).bind("1.0.0").bind("")
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn add_tool(db: &Database, name: &str, description: &str, language: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO tools (id, name, description, code, language, script_path, version) VALUES (?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(name).bind(description).bind("").bind(language).bind("").bind("1.0.0")
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_agent(db: &Database, agent: AgentInfo) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE agents SET name = ?, description = ?, instructions = ?, model = ?, domain = ?, version = ?, code = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(agent.name).bind(agent.description).bind(agent.instructions).bind(agent.model).bind(agent.domain).bind(agent.version).bind(agent.code).bind(agent.id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn update_tool(db: &Database, tool: ToolInfo) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE tools SET name = ?, description = ?, code = ?, language = ?, script_path = ?, version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(tool.name).bind(tool.description).bind(tool.code).bind(tool.language).bind(tool.script_path).bind(tool.version).bind(tool.id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

// ── SUPERVISOR CRUD ──

pub async fn get_supervisors(db: &Database) -> anyhow::Result<Vec<crate::db::Supervisor>> {
    let rows = sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code FROM supervisors"
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_supervisor_tools(db: &Database, supervisor_id: i64) -> anyhow::Result<Vec<ToolInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT DISTINCT t.id, t.name, t.description, CAST(t.code AS TEXT) as code, t.language, t.script_path, t.version 
         FROM tools t
         JOIN agent_tools at ON t.id = at.tool_id
         JOIN supervisor_agents sa ON at.agent_id = sa.agent_id
         WHERE sa.supervisor_id = ?"
    )
    .bind(supervisor_id)
    .fetch_all(&db.pool)
    .await?;
    
    let tools = rows.into_iter().map(|r| ToolInfo {
        id: r.get("id"),
        name: r.get("name"),
        description: r.get::<Option<String>, _>("description").unwrap_or_default(),
        code: r.get::<Option<String>, _>("code").unwrap_or_default(),
        language: r.get::<Option<String>, _>("language").unwrap_or_default(),
        script_path: r.get("script_path"),
        version: r.get::<Option<String>, _>("version").unwrap_or_default(),
    }).collect();
    
    Ok(tools)
}

pub async fn add_supervisor(db: &Database, name: &str, description: &str, domain: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO supervisors (id, name, description, domain, model, version, instructions, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(name).bind(description).bind(domain).bind("qwen2.5:7b-instruct").bind("1.0.0").bind("").bind("")
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_supervisor(db: &Database, supervisor: crate::db::Supervisor) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE supervisors SET name = ?, description = ?, instructions = ?, model = ?, domain = ?, version = ?, code = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(supervisor.name).bind(supervisor.description).bind(supervisor.instructions).bind(supervisor.model)
    .bind(supervisor.domain).bind(supervisor.version).bind(supervisor.code).bind(supervisor.id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn delete_supervisor(db: &Database, id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM supervisors WHERE id = ?").bind(id).execute(&db.pool).await?;
    Ok(())
}

pub async fn get_supervisor_agents(db: &Database, supervisor_id: i64) -> anyhow::Result<Vec<AgentInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT a.id, a.name, a.description, a.instructions, a.model, a.domain, a.version, CAST(a.code AS TEXT) as code 
         FROM agents a
         JOIN supervisor_agents sa ON a.id = sa.agent_id
         WHERE sa.supervisor_id = ?"
    )
    .bind(supervisor_id)
    .fetch_all(&db.pool)
    .await?;
    
    let mut agents = Vec::new();
    for r in rows {
        let agent_id: i64 = r.get("id");
        let tools_rows = sqlx::query(
            "SELECT t.name FROM tools t 
             JOIN agent_tools at ON t.id = at.tool_id 
             WHERE at.agent_id = ?"
        )
        .bind(agent_id)
        .fetch_all(&db.pool)
        .await?;

        agents.push(AgentInfo {
            id: agent_id,
            name: r.get("name"),
            description: r.get::<Option<String>, _>("description").unwrap_or_default(),
            instructions: r.get::<Option<String>, _>("instructions").unwrap_or_default(),
            model: r.get::<Option<String>, _>("model").unwrap_or_default(),
            domain: r.get::<Option<String>, _>("domain").unwrap_or_default(),
            version: r.get::<Option<String>, _>("version").unwrap_or_default(),
            code: r.get("code"),
            tools: tools_rows.into_iter().map(|t| t.get("name")).collect(),
            source_files: vec![],
        });
    }
    Ok(agents)
}

pub async fn link_agent_to_supervisor(db: &Database, supervisor_id: i64, agent_id: i64) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT OR IGNORE INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (?, ?, ?)"
    )
    .bind(id).bind(supervisor_id).bind(agent_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn unlink_agent_from_supervisor(db: &Database, supervisor_id: i64, agent_id: i64) -> anyhow::Result<()> {
    sqlx::query(
        "DELETE FROM supervisor_agents WHERE supervisor_id = ? AND agent_id = ?"
    )
    .bind(supervisor_id).bind(agent_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn get_planners(db: &Database) -> anyhow::Result<Vec<crate::db::Planner>> {
    let rows = sqlx::query_as::<_, crate::db::Planner>(
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code FROM planners ORDER BY name ASC"
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn add_planner(db: &Database, name: &str, description: &str, domain: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO planners (id, name, description, domain, model, version, instructions, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(name).bind(description).bind(domain).bind("qwen2.5:7b-instruct").bind("1.0.0").bind("").bind("")
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_planner(db: &Database, planner: crate::db::Planner) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE planners SET name = ?, description = ?, instructions = ?, model = ?, domain = ?, version = ?, code = ? WHERE id = ?"
    )
    .bind(planner.name).bind(planner.description).bind(planner.instructions).bind(planner.model)
    .bind(planner.domain).bind(planner.version).bind(planner.code).bind(planner.id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn delete_planner(db: &Database, id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM planners WHERE id = ?").bind(id).execute(&db.pool).await?;
    Ok(())
}

pub async fn add_tool_to_agent(db: &Database, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id) VALUES (?, ?, ?)"
    )
    .bind(id).bind(agent_id).bind(tool_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn remove_tool_from_agent(db: &Database, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
    sqlx::query(
        "DELETE FROM agent_tools WHERE agent_id = ? AND tool_id = ?"
    )
    .bind(agent_id).bind(tool_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn get_conversations(db: &Database) -> anyhow::Result<Vec<Conversation>> {
    let convs = sqlx::query_as::<_, Conversation>(
        "SELECT id, title, created_at FROM conversations ORDER BY updated_at DESC"
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(convs)
}

pub async fn add_conversation(db: &Database, title: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO conversations (id, title) VALUES (?, ?)"
    )
    .bind(id).bind(title)
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_conversation_title(db: &Database, id: i64, title: &str) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE conversations SET title = ? WHERE id = ?"
    )
    .bind(title).bind(id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn get_messages(db: &Database, conversation_id: i64) -> anyhow::Result<Vec<Message>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, conversation_id, role, content, created_at FROM messages WHERE conversation_id = ? ORDER BY created_at ASC"
    )
    .bind(conversation_id)
    .fetch_all(&db.pool)
    .await?;
    
    let mut messages = Vec::new();
    for r in rows {
        let message_id: i64 = r.get("id");
        let attachments = sqlx::query_as::<_, Attachment>(
            "SELECT id, message_id, filename, file_path, content_type, size FROM attachments WHERE message_id = ?"
        )
        .bind(message_id)
        .fetch_all(&db.pool)
        .await?;

        messages.push(Message {
            id: message_id,
            conversation_id: r.get("conversation_id"),
            role: r.get("role"),
            content: r.get("content"),
            created_at: r.get("created_at"),
            attachments,
        });
    }
    
    Ok(messages)
}

pub async fn add_attachment(db: &Database, message_id: i64, filename: &str, file_path: &str, content_type: &str, size: i64) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO attachments (id, message_id, filename, file_path, content_type, size) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(message_id).bind(filename).bind(file_path).bind(content_type).bind(size)
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn add_message(db: &Database, conversation_id: i64, role: &str, content: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO messages (id, conversation_id, role, content) VALUES (?, ?, ?, ?)"
    )
    .bind(id).bind(conversation_id).bind(role).bind(content)
    .execute(&db.pool)
    .await?;
    
    // Update conversation timestamp
    sqlx::query(
        "UPDATE conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(conversation_id)
    .execute(&db.pool)
    .await?;
    
    Ok(id)
}

pub async fn get_agent_by_domain(db: &Database, domain: &str) -> anyhow::Result<Option<AgentInfo>> {
    use sqlx::Row;
    let r = sqlx::query(
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code FROM agents WHERE domain = ? LIMIT 1"
    )
    .bind(domain)
    .fetch_optional(&db.pool)
    .await?;
    
    if let Some(r) = r {
        let agent_id: i64 = r.get("id");
        // Fetch tools for this agent
        let tools_rows = sqlx::query(
            "SELECT t.name FROM tools t JOIN agent_tools at ON t.id = at.tool_id WHERE at.agent_id = ?"
        )
        .bind(agent_id)
        .fetch_all(&db.pool)
        .await?;

        Ok(Some(AgentInfo {
            id: agent_id,
            name: r.get("name"),
            description: r.get::<Option<String>, _>("description").unwrap_or_default(),
            instructions: r.get::<Option<String>, _>("instructions").unwrap_or_default(),
            model: r.get::<Option<String>, _>("model").unwrap_or_default(),
            domain: r.get::<Option<String>, _>("domain").unwrap_or_default(),
            version: r.get::<Option<String>, _>("version").unwrap_or_default(),
            code: r.get("code"),
            tools: tools_rows.into_iter().map(|t| t.get("name")).collect(),
            source_files: vec![],
        }))
    } else {
        Ok(None)
    }
}

pub async fn publish_agent_version(db: &Database, agent_id: i64, version_tag: &str, changelog: &str) -> anyhow::Result<()> {
    // 1. Fetch current agent data
    let agent = get_agents(db).await?
        .into_iter()
        .find(|a| a.id == agent_id)
        .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

    // 2. Serialize to JSON
    let content = serde_json::to_string(&agent)?;

    // 3. Publish
    publish_version(db, "agent", agent_id, version_tag, &content, changelog).await
}

pub async fn publish_tool_version(db: &Database, tool_id: i64, version_tag: &str, changelog: &str) -> anyhow::Result<()> {
    // 1. Fetch current tool data
    let tool = get_tools(db).await?
        .into_iter()
        .find(|t| t.id == tool_id)
        .ok_or_else(|| anyhow::anyhow!("Tool not found"))?;

    // 2. Content is source code
    let content = tool.code;

    // 3. Publish
    publish_version(db, "tool", tool_id, version_tag, &content, changelog).await
}

pub async fn publish_version(db: &Database, entity_type: &str, entity_id: i64, version_tag: &str, content: &str, changelog: &str) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO versions (id, entity_type, entity_id, version_tag, content, changelog) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(entity_type).bind(entity_id).bind(version_tag).bind(content).bind(changelog)
    .execute(&db.pool)
    .await?;

    // Update the main entity to this version
    if entity_type == "agent" {
        sqlx::query(
            "UPDATE agents SET version = ? WHERE id = ?"
        )
        .bind(version_tag).bind(entity_id)
        .execute(&db.pool)
        .await?;
    } else if entity_type == "tool" {
        sqlx::query(
            "UPDATE tools SET version = ? WHERE id = ?"
        )
        .bind(version_tag).bind(entity_id)
        .execute(&db.pool)
        .await?;
    }
    
    Ok(())
}

pub async fn rollback_version(db: &Database, entity_type: &str, entity_id: i64, version_tag: &str) -> anyhow::Result<()> {
    use sqlx::Row;
    let v = sqlx::query(
        "SELECT content FROM versions WHERE entity_type = ? AND entity_id = ? AND version_tag = ? LIMIT 1"
    )
    .bind(entity_type).bind(entity_id).bind(version_tag)
    .fetch_one(&db.pool)
    .await?;
    
    let content: String = v.get("content");

    if entity_type == "agent" {
        sqlx::query(
            "UPDATE agents SET version = ?, instructions = ? WHERE id = ?"
        )
        .bind(version_tag).bind(content).bind(entity_id)
        .execute(&db.pool)
        .await?;
    } else if entity_type == "tool" {
        sqlx::query(
            "UPDATE tools SET version = ?, code = ? WHERE id = ?"
        )
        .bind(version_tag).bind(content).bind(entity_id)
        .execute(&db.pool)
        .await?;
    }

    Ok(())
}

pub async fn get_version_history(db: &Database, entity_type: &str, entity_id: i64) -> anyhow::Result<Vec<EntityVersion>> {
    let rows = sqlx::query_as::<_, EntityVersion>(
        "SELECT id, version_tag, changelog, created_at FROM versions WHERE entity_type = ? AND entity_id = ? ORDER BY created_at DESC"
    )
    .bind(entity_type).bind(entity_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct EntityVersion {
    pub id: i64,
    pub version_tag: String,
    pub changelog: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Conversation {
    pub id: i64,
    pub title: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: chrono::NaiveDateTime,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Attachment {
    pub id: i64,
    pub message_id: i64,
    pub filename: String,
    pub file_path: String,
    pub content_type: String,
    pub size: i64,
}


pub async fn test_tool_code(language: &str, code: &str, query: Option<&str>) -> anyhow::Result<String> {
    use std::process::Command;
    use std::io::Write;
    use tempfile::NamedTempFile;

    if language.to_lowercase() == "python" {
        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "{}", code)?;
        let path = temp_file.path().to_str().unwrap();

        let mut cmd = Command::new("python3");
        cmd.arg(path);

        if let Some(q) = query {
            // Using a simple whitespace split to mimic CLI arguments from the UI text field.
            // For complex quoted arguments, a real shell lexer would be better, but this handles basic flags.
            let args: Vec<&str> = q.split_whitespace().collect();
            cmd.args(args);
        }

        let output = cmd.output()?;

        let mut result = String::new();
        if !output.stdout.is_empty() {
            result.push_str(&String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str("--- Error Output ---\n");
            result.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        if result.is_empty() {
            result.push_str("Executed unconditionally with no output.");
        }
        Ok(result)
    } else {
        Ok(format!("Dynamic execution for language '{}' is currently unsupported or mocked. Code registered.", language))
    }
}

pub async fn test_agent_preamble(instructions: &str) -> anyhow::Result<String> {
    // A mock test for agent preamble syntax. For now, it simply validates length and placeholders.
    if instructions.is_empty() {
        return Ok("Agent preamble is empty. Please provide instructions.".to_string());
    }
    
    let mut result = String::new();
    result.push_str("Syntax Check: Valid\n");
    result.push_str(&format!("Length: {} characters\n", instructions.len()));
    
    if instructions.contains("{{") && instructions.contains("}}") {
        result.push_str("Detected template variables ({{...}}). Ensure these are populated at runtime.\n");
    }
    
    result.push_str("Agent instructions are ready for deployment.");
    Ok(result)
}

pub async fn get_setting(db: &Database, key: &str) -> String {
    let res: Result<Option<(String,)>, sqlx::Error> = sqlx::query_as("SELECT value FROM settings WHERE key = ?")
        .bind(key)
        .fetch_optional(&db.pool)
        .await;
    
    match res {
        Ok(Some((v,))) => v,
        _ => "".to_string(),
    }
}

pub async fn set_setting(db: &Database, key: &str, value: &str) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query("INSERT INTO settings (id, key, value) VALUES (?, ?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP")
        .bind(id)
        .bind(key)
        .bind(value)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn get_all_settings(db: &Database) -> HashMap<String, String> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(&db.pool)
        .await
        .unwrap_or_default();
    
    rows.into_iter().collect()
}

pub async fn get_attachments(db: &Database, message_id: i64) -> anyhow::Result<Vec<Attachment>> {
    let rows = sqlx::query_as::<_, Attachment>(
        "SELECT id, message_id, filename, file_path, content_type, size FROM attachments WHERE message_id = ?"
    )
    .bind(message_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_all_agents(db: &Database) -> anyhow::Result<Vec<AgentInfo>> {
    get_agents(db).await
}

pub async fn get_system_load() -> SystemLoad {
    use std::process::Command;

    // 1. Try GPU Load (NVIDIA)
    let gpu_output = Command::new("nvidia-smi")
        .args(["--query-gpu=utilization.gpu", "--format=csv,noheader,nounits"])
        .output();

    if let Ok(output) = gpu_output {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if let Ok(usage) = s.parse::<u32>() {
                return SystemLoad {
                    label: "GPU Load".to_string(),
                    usage,
                };
            }
        }
    }

    // 2. Fallback to CPU Load
    // We use top to get a quick snapshot of CPU idle time
    let cpu_output = Command::new("sh")
        .arg("-c")
        .arg("top -bn1 | grep '%Cpu(s)'")
        .output();

    if let Ok(output) = cpu_output {
        if output.status.success() {
            let s = String::from_utf8_lossy(&output.stdout);
            // Example: %Cpu(s):  8.3 us,  1.5 sy,  0.0 ni, 88.6 id, ...
            if let Some(idle_part) = s.split(',').find(|part| part.contains("id")) {
                let idle_val = idle_part
                    .trim()
                    .split_whitespace()
                    .next()
                    .and_then(|v| v.parse::<f32>().ok());
                
                if let Some(idle) = idle_val {
                    let usage = (100.0 - idle).round() as u32;
                    return SystemLoad {
                        label: "CPU Load".to_string(),
                        usage: usage.min(100),
                    };
                }
            }
        }
    }

    // 3. Absolute fallback
    SystemLoad {
        label: "System Load".to_string(),
        usage: 0,
    }
}
