use crate::engine::utils::discovery::{AgentInfo, ToolInfo};
use crate::db::Database;
use serde::{Deserialize, Serialize};
use reqwest;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaModel {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct OllamaTagsResponse {
    models: Vec<OllamaModel>,
}

pub async fn list_ollama_models() -> Vec<String> {
    let client = reqwest::Client::new();
    match client.get("http://localhost:11434/api/tags").send().await {
        Ok(resp) => {
            if let Ok(json) = resp.json::<OllamaTagsResponse>().await {
                json.models.into_iter().map(|m| m.name).collect()
            } else {
                vec!["llama3".to_string(), "qwen2.5:7b-instruct".to_string()]
            }
        }
        Err(_) => vec!["llama3".to_string(), "qwen2.5:7b-instruct".to_string()],
    }
}

pub async fn get_agents(db: &Database) -> anyhow::Result<Vec<AgentInfo>> {
    let rows = sqlx::query!(
        r#"SELECT id, name, description, instructions, model, domain, version FROM agents"#
    )
    .fetch_all(&db.pool)
    .await?;
    
    let mut agents = Vec::new();
    for r in rows {
        let agent_id = r.id.expect("ID should not be null");
        
        // Fetch tools for this agent
        let tools = sqlx::query!(
            r#"SELECT t.name FROM tools t 
               JOIN agent_tools at ON t.id = at.tool_id 
               WHERE at.agent_id = ?"#,
            agent_id
        )
        .fetch_all(&db.pool)
        .await?;

        agents.push(AgentInfo {
            id: agent_id,
            name: r.name,
            description: r.description.unwrap_or_default(),
            instructions: r.instructions.unwrap_or_default(),
            model: r.model.unwrap_or_default(),
            domain: r.domain.unwrap_or_default(),
            version: r.version.unwrap_or_default(),
            tools: tools.into_iter().map(|t| t.name).collect(),
        });
    }
    
    Ok(agents)
}

pub async fn get_tools(db: &Database) -> anyhow::Result<Vec<ToolInfo>> {
    let rows = sqlx::query!(
        r#"SELECT id, name, description, code, language, script_path, version FROM tools"#
    )
    .fetch_all(&db.pool)
    .await?;
    
    let tools = rows.into_iter().map(|r| ToolInfo {
        id: r.id.expect("ID should not be null"),
        name: r.name,
        description: r.description.unwrap_or_default(),
        code: r.code.unwrap_or_default(),
        language: r.language.unwrap_or_default(),
        script_path: r.script_path,
        version: r.version.unwrap_or_default(),
    }).collect();
    
    Ok(tools)
}

pub async fn add_agent(db: &Database, name: &str, description: &str, domain: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query!(
        "INSERT INTO agents (id, name, description, instructions, model, domain, version) VALUES (?, ?, ?, ?, ?, ?, ?)",
        id, name, description, "", "llama3", domain, "1.0.0"
    )
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn add_tool(db: &Database, name: &str, description: &str, language: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query!(
        "INSERT INTO tools (id, name, description, code, language, script_path, version) VALUES (?, ?, ?, ?, ?, ?, ?)",
        id, name, description, "", language, "", "1.0.0"
    )
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_agent(db: &Database, agent: AgentInfo) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE agents SET name = ?, description = ?, instructions = ?, model = ?, domain = ?, version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        agent.name, agent.description, agent.instructions, agent.model, agent.domain, agent.version, agent.id
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn update_tool(db: &Database, tool: ToolInfo) -> anyhow::Result<()> {
    sqlx::query!(
        "UPDATE tools SET name = ?, description = ?, code = ?, language = ?, script_path = ?, version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        tool.name, tool.description, tool.code, tool.language, tool.script_path, tool.version, tool.id
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn add_tool_to_agent(db: &Database, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
    sqlx::query!(
        "INSERT OR IGNORE INTO agent_tools (agent_id, tool_id) VALUES (?, ?)",
        agent_id, tool_id
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn remove_tool_from_agent(db: &Database, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
    sqlx::query!(
        "DELETE FROM agent_tools WHERE agent_id = ? AND tool_id = ?",
        agent_id, tool_id
    )
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn get_conversations(db: &Database) -> anyhow::Result<Vec<Conversation>> {
    let convs = sqlx::query_as!(
        Conversation,
        r#"SELECT id as "id!", title, created_at as "created_at!" FROM conversations ORDER BY updated_at DESC"#
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(convs)
}

pub async fn add_conversation(db: &Database, title: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query!(
        "INSERT INTO conversations (id, title) VALUES (?, ?)",
        id, title
    )
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn get_messages(db: &Database, conversation_id: i64) -> anyhow::Result<Vec<Message>> {
    let rows = sqlx::query!(
        r#"SELECT id as "id!", conversation_id as "conversation_id!", role as "role!", content as "content!", created_at as "created_at!" FROM messages WHERE conversation_id = ? ORDER BY created_at ASC"#,
        conversation_id
    )
    .fetch_all(&db.pool)
    .await?;
    
    let mut messages = Vec::new();
    for r in rows {
        let message_id = r.id;
        let attachments = sqlx::query_as!(
            Attachment,
            r#"SELECT id as "id!", message_id as "message_id!", filename as "filename!", file_path as "file_path!", content_type as "content_type!", size as "size!" FROM attachments WHERE message_id = ?"#,
            message_id
        )
        .fetch_all(&db.pool)
        .await?;

        messages.push(Message {
            id: r.id,
            conversation_id: r.conversation_id,
            role: r.role,
            content: r.content,
            created_at: r.created_at,
            attachments,
        });
    }
    
    Ok(messages)
}

pub async fn add_attachment(db: &Database, message_id: i64, filename: &str, file_path: &str, content_type: &str, size: i64) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query!(
        "INSERT INTO attachments (id, message_id, filename, file_path, content_type, size) VALUES (?, ?, ?, ?, ?, ?)",
        id, message_id, filename, file_path, content_type, size
    )
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn add_message(db: &Database, conversation_id: i64, role: &str, content: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query!(
        "INSERT INTO messages (id, conversation_id, role, content) VALUES (?, ?, ?, ?)",
        id, conversation_id, role, content
    )
    .execute(&db.pool)
    .await?;
    
    // Update conversation timestamp
    sqlx::query!(
        "UPDATE conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = ?",
        conversation_id
    )
    .execute(&db.pool)
    .await?;
    
    Ok(id)
}

pub async fn get_agent_by_domain(db: &Database, domain: &str) -> anyhow::Result<Option<AgentInfo>> {
    let r = sqlx::query!(
        r#"SELECT id, name, description, instructions, model, domain, version FROM agents WHERE domain = ? LIMIT 1"#,
        domain
    )
    .fetch_optional(&db.pool)
    .await?;
    
    if let Some(r) = r {
        // Fetch tools for this agent
        let tools = sqlx::query!(
            r#"SELECT t.name FROM tools t JOIN agent_tools at ON t.id = at.tool_id WHERE at.agent_id = ?"#,
            r.id
        )
        .fetch_all(&db.pool)
        .await?;

        Ok(Some(AgentInfo {
            id: r.id.expect("ID should not be null"),
            name: r.name,
            description: r.description.unwrap_or_default(),
            instructions: r.instructions.unwrap_or_default(),
            model: r.model.unwrap_or_default(),
            domain: r.domain.unwrap_or_default(),
            version: r.version.unwrap_or_default(),
            tools: tools.into_iter().map(|t| t.name).collect(),
        }))
    } else {
        Ok(None)
    }
}

pub async fn publish_version(db: &Database, entity_type: &str, entity_id: i64, version_tag: &str, content: &str, changelog: &str) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query!(
        "INSERT INTO versions (id, entity_type, entity_id, version_tag, content, changelog) VALUES (?, ?, ?, ?, ?, ?)",
        id, entity_type, entity_id, version_tag, content, changelog
    )
    .execute(&db.pool)
    .await?;

    // Update the main entity to this version
    if entity_type == "agent" {
        sqlx::query!(
            "UPDATE agents SET version = ?, instructions = ? WHERE id = ?",
            version_tag, content, entity_id
        )
        .execute(&db.pool)
        .await?;
    } else if entity_type == "tool" {
        sqlx::query!(
            "UPDATE tools SET version = ?, code = ? WHERE id = ?",
            version_tag, content, entity_id
        )
        .execute(&db.pool)
        .await?;
    }
    
    Ok(())
}

pub async fn rollback_version(db: &Database, entity_type: &str, entity_id: i64, version_tag: &str) -> anyhow::Result<()> {
    let v = sqlx::query!(
        "SELECT content FROM versions WHERE entity_type = ? AND entity_id = ? AND version_tag = ? LIMIT 1",
        entity_type, entity_id, version_tag
    )
    .fetch_one(&db.pool)
    .await?;

    if entity_type == "agent" {
        sqlx::query!(
            "UPDATE agents SET version = ?, instructions = ? WHERE id = ?",
            version_tag, v.content, entity_id
        )
        .execute(&db.pool)
        .await?;
    } else if entity_type == "tool" {
        sqlx::query!(
            "UPDATE tools SET version = ?, code = ? WHERE id = ?",
            version_tag, v.content, entity_id
        )
        .execute(&db.pool)
        .await?;
    }

    Ok(())
}

pub async fn get_version_history(db: &Database, entity_type: &str, entity_id: i64) -> anyhow::Result<Vec<EntityVersion>> {
    let rows = sqlx::query_as!(
        EntityVersion,
        r#"SELECT id as "id!", version_tag, changelog, created_at as "created_at!" FROM versions WHERE entity_type = ? AND entity_id = ? ORDER BY created_at DESC"#,
        entity_type, entity_id
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityVersion {
    pub id: i64,
    pub version_tag: String,
    pub changelog: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: i64,
    pub title: String,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: chrono::NaiveDateTime,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: i64,
    pub message_id: i64,
    pub filename: String,
    pub file_path: String,
    pub content_type: String,
    pub size: i64,
}
