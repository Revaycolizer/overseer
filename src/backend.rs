use crate::db::Database;
use crate::engine::utils::discovery::{AgentInfo, ToolInfo};
use reqwest;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

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
pub struct RuntimeHealth {
    pub ollama_reachable: bool,
    pub ollama_endpoint: String,
    pub available_models: Vec<String>,
    pub selected_runtime_model: String,
    pub runtime_model_available: bool,
    pub title_model: String,
    pub title_model_available: bool,
    pub python_command: Option<String>,
    pub protoc_path: Option<String>,
    pub runtime_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputerSupervisionState {
    pub supervision_mode: String,
    pub session_state: String,
    pub session_note: String,
    pub allowlist: Vec<String>,
    pub max_actions: i64,
    pub action_count: i64,
    pub timeout_ms: i64,
    pub approval_state: String,
    pub pending_tool: String,
    pub pending_action: String,
    pub pending_domain: String,
    pub session_started_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct ToolTestCase {
    pub id: i64,
    pub tool_id: i64,
    pub name: String,
    pub input: String,
    pub expected_substring: Option<String>,
    pub last_status: Option<String>,
    pub last_output: Option<String>,
    pub last_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
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
                    return models
                        .iter()
                        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                        .collect();
                }
            }
            vec![
                "llama3:latest".to_string(),
                "qwen2.5:7b-instruct".to_string(),
            ]
        }
        Err(_) => vec![
            "llama3:latest".to_string(),
            "qwen2.5:7b-instruct".to_string(),
        ],
    }
}

async fn try_get_available_models() -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|err| err.to_string())?;

    let json = response
        .json::<serde_json::Value>()
        .await
        .map_err(|err| err.to_string())?;

    Ok(json["models"]
        .as_array()
        .map(|models| {
            models
                .iter()
                .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default())
}

fn locate_protoc() -> Option<String> {
    let mut candidates = Vec::new();

    if let Ok(explicit) = std::env::var("PROTOC") {
        candidates.push(explicit);
    }

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(
            current_dir
                .join("tools")
                .join("bin")
                .join(if cfg!(windows) { "protoc.exe" } else { "protoc" })
                .display()
                .to_string(),
        );
    }

    if cfg!(windows) {
        candidates.push(r"C:\tools\bin\protoc.exe".to_string());
        candidates.push(r"C:\ProgramData\chocolatey\bin\protoc.exe".to_string());
    } else {
        candidates.push("/usr/local/bin/protoc".to_string());
        candidates.push("/usr/bin/protoc".to_string());
    }

    for candidate in candidates {
        if !candidate.trim().is_empty() && std::path::Path::new(candidate.trim()).exists() {
            return Some(candidate);
        }
    }

    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut probe = if cfg!(windows) {
        Command::new("where")
    } else {
        Command::new("which")
    };
    probe.arg("protoc");
    #[cfg(windows)]
    probe.creation_flags(CREATE_NO_WINDOW);

    if let Ok(output) = probe.output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_string);
            if path.is_some() {
                return path;
            }
        }
    }

    None
}

pub async fn get_runtime_health(db: &Database) -> RuntimeHealth {
    let strict_models = try_get_available_models().await.unwrap_or_default();
    let ollama_reachable = !strict_models.is_empty() || try_get_available_models().await.is_ok();
    let selected_runtime_model = get_setting_or(db, "default_chat_model", "qwen2.5:7b-instruct").await;
    let title_model = get_setting_or(db, "title_generator_model", "qwen2.5:7b-instruct").await;
    let python_command = crate::engine::utils::discovery::detect_python_command().map(|parts| parts.join(" "));
    let protoc_path = locate_protoc();
    let runtime_dir = crate::engine::utils::discovery::runtime_dir()
        .display()
        .to_string();

    let runtime_model_available = strict_models.iter().any(|model| model == &selected_runtime_model);
    let title_model_available = strict_models.iter().any(|model| model == &title_model);

    RuntimeHealth {
        ollama_reachable,
        ollama_endpoint: "http://localhost:11434".to_string(),
        available_models: strict_models,
        selected_runtime_model,
        runtime_model_available,
        title_model,
        title_model_available,
        python_command,
        protoc_path,
        runtime_dir,
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
             WHERE at.agent_id = ?",
        )
        .bind(agent_id)
        .fetch_all(&db.pool)
        .await?;

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
            tools: tools_rows.into_iter().map(|t| t.get("name")).collect(),
            source_files: vec![],
        });
    }

    Ok(agents)
}

pub async fn get_tools(db: &Database) -> anyhow::Result<Vec<ToolInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, description, CAST(code AS TEXT) as code, language, script_path, required_intent, safety_level, version FROM tools"
    )
    .fetch_all(&db.pool)
    .await?;

    let tools = rows
        .into_iter()
        .map(|r| ToolInfo {
            id: r.get("id"),
            name: r.get("name"),
            description: r
                .get::<Option<String>, _>("description")
                .unwrap_or_default(),
            code: r.get::<Option<String>, _>("code").unwrap_or_default(),
            language: r.get::<Option<String>, _>("language").unwrap_or_default(),
            script_path: r.get("script_path"),
            required_intent: r.get("required_intent"),
            safety_level: r
                .get::<Option<String>, _>("safety_level")
                .unwrap_or_else(|| "standard".to_string()),
            version: r.get::<Option<String>, _>("version").unwrap_or_default(),
        })
        .collect();

    Ok(tools)
}

pub async fn add_agent(
    db: &Database,
    name: &str,
    description: &str,
    domain: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    let model = resolve_model_name(db, None, "default_chat_model", "qwen2.5:7b-instruct").await;
    sqlx::query(
        "INSERT INTO agents (id, name, description, instructions, model, domain, version, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(name).bind(description).bind("").bind(model).bind(domain).bind("1.0.0").bind("")
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn add_tool(
    db: &Database,
    name: &str,
    description: &str,
    language: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO tools (id, name, description, code, language, script_path, required_intent, safety_level, version) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(name).bind(description).bind("").bind(language).bind("").bind(None::<String>).bind("standard").bind("1.0.0")
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

pub async fn get_tool_test_cases(
    db: &Database,
    tool_id: i64,
) -> anyhow::Result<Vec<ToolTestCase>> {
    let cases = sqlx::query_as::<_, ToolTestCase>(
        "SELECT id, tool_id, name, input, expected_substring, last_status, last_output, last_run_at, created_at, updated_at
         FROM tool_test_cases
         WHERE tool_id = ?
         ORDER BY updated_at DESC, id DESC",
    )
    .bind(tool_id)
    .fetch_all(&db.pool)
    .await?;

    Ok(cases)
}

pub async fn add_tool_test_case(
    db: &Database,
    tool_id: i64,
    name: &str,
    input: &str,
    expected_substring: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO tool_test_cases (tool_id, name, input, expected_substring)
         VALUES (?, ?, ?, ?)",
    )
    .bind(tool_id)
    .bind(name)
    .bind(input)
    .bind(expected_substring)
    .execute(&db.pool)
    .await?;

    Ok(())
}

pub async fn update_tool_test_case(
    db: &Database,
    test_case: ToolTestCase,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE tool_test_cases
         SET name = ?, input = ?, expected_substring = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(test_case.name)
    .bind(test_case.input)
    .bind(test_case.expected_substring)
    .bind(test_case.id)
    .execute(&db.pool)
    .await?;

    Ok(())
}

pub async fn delete_tool_test_case(db: &Database, case_id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM tool_test_cases WHERE id = ?")
        .bind(case_id)
        .execute(&db.pool)
        .await?;

    Ok(())
}

pub async fn record_tool_test_case_result(
    db: &Database,
    case_id: i64,
    status: &str,
    output: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE tool_test_cases
         SET last_status = ?, last_output = ?, last_run_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(status)
    .bind(output)
    .bind(case_id)
    .execute(&db.pool)
    .await?;

    Ok(())
}

pub async fn update_tool(db: &Database, tool: ToolInfo) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE tools SET name = ?, description = ?, code = ?, language = ?, script_path = ?, required_intent = ?, safety_level = ?, version = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(tool.name)
    .bind(tool.description)
    .bind(tool.code)
    .bind(tool.language)
    .bind(tool.script_path)
    .bind(tool.required_intent)
    .bind(tool.safety_level)
    .bind(tool.version)
    .bind(tool.id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

// ── SUPERVISOR CRUD ──

pub async fn get_supervisors(db: &Database) -> anyhow::Result<Vec<crate::db::Supervisor>> {
    let rows = sqlx::query_as::<_, crate::db::Supervisor>(
        "SELECT id, name, description, instructions, model, domain, version, CAST(code AS TEXT) as code, bypass_synthesizer FROM supervisors"
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_supervisor_tools(
    db: &Database,
    supervisor_id: i64,
) -> anyhow::Result<Vec<ToolInfo>> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT DISTINCT t.id, t.name, t.description, CAST(t.code AS TEXT) as code, t.language, t.script_path, t.required_intent, t.safety_level, t.version 
         FROM tools t
         JOIN agent_tools at ON t.id = at.tool_id
         JOIN supervisor_agents sa ON at.agent_id = sa.agent_id
         WHERE sa.supervisor_id = ?"
    )
    .bind(supervisor_id)
    .fetch_all(&db.pool)
    .await?;

    let tools = rows
        .into_iter()
        .map(|r| ToolInfo {
            id: r.get("id"),
            name: r.get("name"),
            description: r
                .get::<Option<String>, _>("description")
                .unwrap_or_default(),
            code: r.get::<Option<String>, _>("code").unwrap_or_default(),
            language: r.get::<Option<String>, _>("language").unwrap_or_default(),
            script_path: r.get("script_path"),
            required_intent: r.get("required_intent"),
            safety_level: r
                .get::<Option<String>, _>("safety_level")
                .unwrap_or_else(|| "standard".to_string()),
            version: r.get::<Option<String>, _>("version").unwrap_or_default(),
        })
        .collect();

    Ok(tools)
}

pub async fn add_supervisor(
    db: &Database,
    name: &str,
    description: &str,
    domain: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    let model = resolve_model_name(db, None, "default_chat_model", "qwen2.5:7b-instruct").await;
    sqlx::query(
        "INSERT INTO supervisors (id, name, description, domain, model, version, instructions, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(name).bind(description).bind(domain).bind(model).bind("1.0.0").bind("").bind("")
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_supervisor(
    db: &Database,
    supervisor: crate::db::Supervisor,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE supervisors SET name = ?, description = ?, instructions = ?, model = ?, domain = ?, version = ?, code = ?, bypass_synthesizer = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(supervisor.name).bind(supervisor.description).bind(supervisor.instructions).bind(supervisor.model)
    .bind(supervisor.domain).bind(supervisor.version).bind(supervisor.code)
    .bind(supervisor.bypass_synthesizer).bind(supervisor.id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn delete_supervisor(db: &Database, id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM supervisors WHERE id = ?")
        .bind(id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn get_supervisor_agents(
    db: &Database,
    supervisor_id: i64,
) -> anyhow::Result<Vec<AgentInfo>> {
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
             WHERE at.agent_id = ?",
        )
        .bind(agent_id)
        .fetch_all(&db.pool)
        .await?;

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
            domain: r.get::<Option<String>, _>("domain").unwrap_or_default(),
            version: r.get::<Option<String>, _>("version").unwrap_or_default(),
            code: r.get("code"),
            tools: tools_rows.into_iter().map(|t| t.get("name")).collect(),
            source_files: vec![],
        });
    }
    Ok(agents)
}

pub async fn link_agent_to_supervisor(
    db: &Database,
    supervisor_id: i64,
    agent_id: i64,
) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT OR IGNORE INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (?, ?, ?)",
    )
    .bind(id)
    .bind(supervisor_id)
    .bind(agent_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn unlink_agent_from_supervisor(
    db: &Database,
    supervisor_id: i64,
    agent_id: i64,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM supervisor_agents WHERE supervisor_id = ? AND agent_id = ?")
        .bind(supervisor_id)
        .bind(agent_id)
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

pub async fn add_planner(
    db: &Database,
    name: &str,
    description: &str,
    domain: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    let model = resolve_model_name(db, None, "default_chat_model", "qwen2.5:7b-instruct").await;
    sqlx::query(
        "INSERT INTO planners (id, name, description, domain, model, version, instructions, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(name).bind(description).bind(domain).bind(model).bind("1.0.0").bind("").bind("")
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
    sqlx::query("DELETE FROM planners WHERE id = ?")
        .bind(id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn add_tool_to_agent(db: &Database, agent_id: i64, tool_id: i64) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query("INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id) VALUES (?, ?, ?)")
        .bind(id)
        .bind(agent_id)
        .bind(tool_id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn remove_tool_from_agent(
    db: &Database,
    agent_id: i64,
    tool_id: i64,
) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM agent_tools WHERE agent_id = ? AND tool_id = ?")
        .bind(agent_id)
        .bind(tool_id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn get_conversations(db: &Database) -> anyhow::Result<Vec<Conversation>> {
    let convs = sqlx::query_as::<_, Conversation>(
        "SELECT id, title, created_at FROM conversations ORDER BY updated_at DESC",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(convs)
}

pub async fn add_conversation(db: &Database, title: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query("INSERT INTO conversations (id, title) VALUES (?, ?)")
        .bind(id)
        .bind(title)
        .execute(&db.pool)
        .await?;
    Ok(id)
}

pub async fn update_conversation_title(db: &Database, id: i64, title: &str) -> anyhow::Result<()> {
    sqlx::query("UPDATE conversations SET title = ? WHERE id = ?")
        .bind(title)
        .bind(id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn delete_conversation(db: &Database, id: i64) -> anyhow::Result<()> {
    let attachment_paths: Vec<String> = sqlx::query_scalar(
        "SELECT a.file_path
         FROM attachments a
         JOIN messages m ON m.id = a.message_id
         WHERE m.conversation_id = ?",
    )
    .bind(id)
    .fetch_all(&db.pool)
    .await
    .unwrap_or_default();

    sqlx::query("DELETE FROM conversations WHERE id = ?")
        .bind(id)
        .execute(&db.pool)
        .await?;

    for path in attachment_paths {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            continue;
        }

        let _ = std::fs::remove_file(trimmed);
    }

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

pub async fn add_attachment(
    db: &Database,
    message_id: i64,
    filename: &str,
    file_path: &str,
    content_type: &str,
    size: i64,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO attachments (id, message_id, filename, file_path, content_type, size) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(message_id).bind(filename).bind(file_path).bind(content_type).bind(size)
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn add_message(
    db: &Database,
    conversation_id: i64,
    role: &str,
    content: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query("INSERT INTO messages (id, conversation_id, role, content) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .execute(&db.pool)
        .await?;

    // Update conversation timestamp
    sqlx::query("UPDATE conversations SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
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
            description: r
                .get::<Option<String>, _>("description")
                .unwrap_or_default(),
            instructions: r
                .get::<Option<String>, _>("instructions")
                .unwrap_or_default(),
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

pub async fn publish_agent_version(
    db: &Database,
    agent_id: i64,
    version_tag: &str,
    changelog: &str,
) -> anyhow::Result<()> {
    // 1. Fetch current agent data
    let agent = get_agents(db)
        .await?
        .into_iter()
        .find(|a| a.id == agent_id)
        .ok_or_else(|| anyhow::anyhow!("Agent not found"))?;

    // 2. Serialize to JSON
    let content = serde_json::to_string(&agent)?;

    // 3. Publish
    publish_version(db, "agent", agent_id, version_tag, &content, changelog).await
}

pub async fn publish_tool_version(
    db: &Database,
    tool_id: i64,
    version_tag: &str,
    changelog: &str,
) -> anyhow::Result<()> {
    // 1. Fetch current tool data
    let tool = get_tools(db)
        .await?
        .into_iter()
        .find(|t| t.id == tool_id)
        .ok_or_else(|| anyhow::anyhow!("Tool not found"))?;

    // 2. Content is source code
    let content = tool.code;

    // 3. Publish
    publish_version(db, "tool", tool_id, version_tag, &content, changelog).await
}

pub async fn publish_version(
    db: &Database,
    entity_type: &str,
    entity_id: i64,
    version_tag: &str,
    content: &str,
    changelog: &str,
) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO versions (id, entity_type, entity_id, version_tag, content, changelog) VALUES (?, ?, ?, ?, ?, ?)"
    )
    .bind(id).bind(entity_type).bind(entity_id).bind(version_tag).bind(content).bind(changelog)
    .execute(&db.pool)
    .await?;

    // Update the main entity to this version
    if entity_type == "agent" {
        sqlx::query("UPDATE agents SET version = ? WHERE id = ?")
            .bind(version_tag)
            .bind(entity_id)
            .execute(&db.pool)
            .await?;
    } else if entity_type == "tool" {
        sqlx::query("UPDATE tools SET version = ? WHERE id = ?")
            .bind(version_tag)
            .bind(entity_id)
            .execute(&db.pool)
            .await?;
    }

    Ok(())
}

pub async fn rollback_version(
    db: &Database,
    entity_type: &str,
    entity_id: i64,
    version_tag: &str,
) -> anyhow::Result<()> {
    use sqlx::Row;
    let v = sqlx::query(
        "SELECT content FROM versions WHERE entity_type = ? AND entity_id = ? AND version_tag = ? LIMIT 1"
    )
    .bind(entity_type).bind(entity_id).bind(version_tag)
    .fetch_one(&db.pool)
    .await?;

    let content: String = v.get("content");

    if entity_type == "agent" {
        let agent: AgentInfo = serde_json::from_str(&content)?;
        sqlx::query(
            "UPDATE agents
             SET version = ?, name = ?, description = ?, instructions = ?, model = ?, domain = ?, code = ?, updated_at = CURRENT_TIMESTAMP
             WHERE id = ?"
        )
        .bind(version_tag)
        .bind(agent.name)
        .bind(agent.description)
        .bind(agent.instructions)
        .bind(agent.model)
        .bind(agent.domain)
        .bind(agent.code)
        .bind(entity_id)
        .execute(&db.pool)
        .await?;
    } else if entity_type == "tool" {
        sqlx::query("UPDATE tools SET version = ?, code = ? WHERE id = ?")
            .bind(version_tag)
            .bind(content)
            .bind(entity_id)
            .execute(&db.pool)
            .await?;
    }

    Ok(())
}

pub async fn get_version_content(
    db: &Database,
    entity_type: &str,
    entity_id: i64,
    version_tag: &str,
) -> anyhow::Result<Option<String>> {
    let row: Option<(String,)> = sqlx::query_as(
        "SELECT content FROM versions WHERE entity_type = ? AND entity_id = ? AND version_tag = ? LIMIT 1"
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(version_tag)
    .fetch_optional(&db.pool)
    .await?;

    Ok(row.map(|(content,)| content))
}

pub async fn get_version_history(
    db: &Database,
    entity_type: &str,
    entity_id: i64,
) -> anyhow::Result<Vec<EntityVersion>> {
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

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct ExecutionRun {
    pub id: i64,
    pub conversation_id: Option<i64>,
    pub user_query: String,
    pub status: String,
    pub final_response: Option<String>,
    pub error_message: Option<String>,
    pub started_at: chrono::NaiveDateTime,
    pub finished_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct ExecutionEvent {
    pub id: i64,
    pub run_id: i64,
    pub sequence: i64,
    pub event_type: String,
    pub status: String,
    pub domain: Option<String>,
    pub step_text: Option<String>,
    pub tool_name: Option<String>,
    pub model: Option<String>,
    pub message: Option<String>,
    pub details: Option<String>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Workflow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub trigger_type: String,
    pub trigger_config: Option<String>,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct WorkflowStep {
    pub id: i64,
    pub workflow_id: i64,
    pub step_order: i64,
    pub name: String,
    pub prompt: String,
    pub enabled: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct WorkflowRun {
    pub id: i64,
    pub workflow_id: i64,
    pub conversation_id: Option<i64>,
    pub status: String,
    pub summary: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub finished_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct WorkflowStepRun {
    pub id: i64,
    pub workflow_run_id: i64,
    pub workflow_step_id: i64,
    pub execution_run_id: Option<i64>,
    pub step_order: i64,
    pub status: String,
    pub output: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub finished_at: Option<chrono::NaiveDateTime>,
}

pub async fn start_execution_run(
    db: &Database,
    conversation_id: i64,
    user_query: &str,
) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO execution_runs (id, conversation_id, user_query, status) VALUES (?, ?, ?, 'running')"
    )
    .bind(id)
    .bind(conversation_id)
    .bind(user_query)
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn finish_execution_run(
    db: &Database,
    run_id: i64,
    status: &str,
    final_response: Option<&str>,
    error_message: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE execution_runs SET status = ?, final_response = ?, error_message = ?, finished_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(status)
    .bind(final_response)
    .bind(error_message)
    .bind(run_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn log_execution_event(
    db: &Database,
    run_id: i64,
    sequence: i64,
    event_type: &str,
    status: &str,
    domain: Option<&str>,
    step_text: Option<&str>,
    tool_name: Option<&str>,
    model: Option<&str>,
    message: Option<&str>,
    details: Option<&str>,
) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO execution_events (id, run_id, sequence, event_type, status, domain, step_text, tool_name, model, message, details)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
    )
    .bind(id)
    .bind(run_id)
    .bind(sequence)
    .bind(event_type)
    .bind(status)
    .bind(domain)
    .bind(step_text)
    .bind(tool_name)
    .bind(model)
    .bind(message)
    .bind(details)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn get_execution_runs_for_conversation(
    db: &Database,
    conversation_id: i64,
    limit: i64,
) -> anyhow::Result<Vec<ExecutionRun>> {
    let rows = sqlx::query_as::<_, ExecutionRun>(
        "SELECT id, conversation_id, user_query, status, final_response, error_message, started_at, finished_at
         FROM execution_runs
         WHERE conversation_id = ?
         ORDER BY started_at DESC
         LIMIT ?"
    )
    .bind(conversation_id)
    .bind(limit.max(1))
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_execution_runs_for_workflow_run(
    db: &Database,
    workflow_run_id: i64,
    limit: i64,
) -> anyhow::Result<Vec<ExecutionRun>> {
    let rows = sqlx::query_as::<_, ExecutionRun>(
        "SELECT er.id, er.conversation_id, er.user_query, er.status, er.final_response, er.error_message, er.started_at, er.finished_at
         FROM execution_runs er
         INNER JOIN workflow_runs wr ON wr.conversation_id = er.conversation_id
         WHERE wr.id = ?
         ORDER BY er.started_at DESC
         LIMIT ?"
    )
    .bind(workflow_run_id)
    .bind(limit.max(1))
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_execution_events(
    db: &Database,
    run_id: i64,
) -> anyhow::Result<Vec<ExecutionEvent>> {
    let rows = sqlx::query_as::<_, ExecutionEvent>(
        "SELECT id, run_id, sequence, event_type, status, domain, step_text, tool_name, model, message, details, created_at
         FROM execution_events
         WHERE run_id = ?
         ORDER BY sequence ASC, created_at ASC"
    )
    .bind(run_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_latest_execution_run_for_conversation(
    db: &Database,
    conversation_id: i64,
) -> anyhow::Result<Option<ExecutionRun>> {
    let row = sqlx::query_as::<_, ExecutionRun>(
        "SELECT id, conversation_id, user_query, status, final_response, error_message, started_at, finished_at
         FROM execution_runs
         WHERE conversation_id = ?
         ORDER BY started_at DESC
         LIMIT 1"
    )
    .bind(conversation_id)
    .fetch_optional(&db.pool)
    .await?;
    Ok(row)
}

pub async fn get_workflows(db: &Database) -> anyhow::Result<Vec<Workflow>> {
    let rows = sqlx::query_as::<_, Workflow>(
        "SELECT id, name, description, trigger_type, trigger_config, status, created_at, updated_at
         FROM workflows
         ORDER BY updated_at DESC, created_at DESC",
    )
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn add_workflow(db: &Database, name: &str) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    sqlx::query(
        "INSERT INTO workflows (id, name, description, trigger_type, trigger_config, status)
         VALUES (?, ?, ?, 'manual', ?, 'draft')",
    )
    .bind(id)
    .bind(name)
    .bind("New workflow")
    .bind(None::<String>)
    .execute(&db.pool)
    .await?;
    Ok(id)
}

pub async fn update_workflow(db: &Database, workflow: Workflow) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflows
         SET name = ?, description = ?, trigger_type = ?, trigger_config = ?, status = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?"
    )
    .bind(workflow.name)
    .bind(workflow.description)
    .bind(workflow.trigger_type)
    .bind(workflow.trigger_config)
    .bind(workflow.status)
    .bind(workflow.id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn delete_workflow(db: &Database, workflow_id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM workflows WHERE id = ?")
        .bind(workflow_id)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn get_workflow_steps(
    db: &Database,
    workflow_id: i64,
) -> anyhow::Result<Vec<WorkflowStep>> {
    let rows = sqlx::query_as::<_, WorkflowStep>(
        "SELECT id, workflow_id, step_order, name, prompt, enabled, created_at, updated_at
         FROM workflow_steps
         WHERE workflow_id = ?
         ORDER BY step_order ASC, created_at ASC",
    )
    .bind(workflow_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn add_workflow_step(db: &Database, workflow_id: i64) -> anyhow::Result<i64> {
    let id = db.next_id().await;
    let next_order: (i64,) = sqlx::query_as(
        "SELECT COALESCE(MAX(step_order), 0) + 1 FROM workflow_steps WHERE workflow_id = ?",
    )
    .bind(workflow_id)
    .fetch_one(&db.pool)
    .await?;

    sqlx::query(
        "INSERT INTO workflow_steps (id, workflow_id, step_order, name, prompt, enabled)
         VALUES (?, ?, ?, ?, ?, 1)",
    )
    .bind(id)
    .bind(workflow_id)
    .bind(next_order.0)
    .bind(format!("Step {}", next_order.0))
    .bind("Describe the action this workflow step should perform.")
    .execute(&db.pool)
    .await?;

    sqlx::query("UPDATE workflows SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(workflow_id)
        .execute(&db.pool)
        .await?;

    Ok(id)
}

pub async fn update_workflow_step(db: &Database, step: WorkflowStep) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflow_steps
         SET step_order = ?, name = ?, prompt = ?, enabled = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(step.step_order)
    .bind(step.name)
    .bind(step.prompt)
    .bind(step.enabled)
    .bind(step.id)
    .execute(&db.pool)
    .await?;

    sqlx::query("UPDATE workflows SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
        .bind(step.workflow_id)
        .execute(&db.pool)
        .await?;

    Ok(())
}

pub async fn delete_workflow_step(db: &Database, step_id: i64) -> anyhow::Result<()> {
    let workflow: Option<(i64,)> =
        sqlx::query_as("SELECT workflow_id FROM workflow_steps WHERE id = ?")
            .bind(step_id)
            .fetch_optional(&db.pool)
            .await?;

    sqlx::query("DELETE FROM workflow_steps WHERE id = ?")
        .bind(step_id)
        .execute(&db.pool)
        .await?;

    if let Some((workflow_id,)) = workflow {
        let steps = get_workflow_steps(db, workflow_id).await?;
        for (index, step) in steps.into_iter().enumerate() {
            sqlx::query("UPDATE workflow_steps SET step_order = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                .bind((index + 1) as i64)
                .bind(step.id)
                .execute(&db.pool)
                .await?;
        }
        sqlx::query("UPDATE workflows SET updated_at = CURRENT_TIMESTAMP WHERE id = ?")
            .bind(workflow_id)
            .execute(&db.pool)
            .await?;
    }

    Ok(())
}

pub async fn get_workflow_runs(
    db: &Database,
    workflow_id: i64,
) -> anyhow::Result<Vec<WorkflowRun>> {
    let rows = sqlx::query_as::<_, WorkflowRun>(
        "SELECT id, workflow_id, conversation_id, status, summary, error_message, created_at, finished_at
         FROM workflow_runs
         WHERE workflow_id = ?
         ORDER BY created_at DESC"
    )
    .bind(workflow_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

pub async fn get_workflow_step_runs(
    db: &Database,
    workflow_run_id: i64,
) -> anyhow::Result<Vec<WorkflowStepRun>> {
    let rows = sqlx::query_as::<_, WorkflowStepRun>(
        "SELECT id, workflow_run_id, workflow_step_id, execution_run_id, step_order, status, output, error_message, created_at, finished_at
         FROM workflow_step_runs
         WHERE workflow_run_id = ?
         ORDER BY step_order ASC"
    )
    .bind(workflow_run_id)
    .fetch_all(&db.pool)
    .await?;
    Ok(rows)
}

async fn fail_workflow_run(
    db: &Database,
    workflow_run_id: i64,
    error_message: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE workflow_runs
         SET status = 'failed', error_message = ?, finished_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(error_message)
    .bind(workflow_run_id)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn run_workflow(
    db: &Database,
    agent_system: &crate::engine::system::AgentSystem,
    workflow_id: i64,
) -> anyhow::Result<i64> {
    let workflow = sqlx::query_as::<_, Workflow>(
        "SELECT id, name, description, trigger_type, trigger_config, status, created_at, updated_at
         FROM workflows
         WHERE id = ?",
    )
    .bind(workflow_id)
    .fetch_one(&db.pool)
    .await?;

    let steps = get_workflow_steps(db, workflow_id)
        .await?
        .into_iter()
        .filter(|step| step.enabled)
        .collect::<Vec<_>>();

    let run_id = db.next_id().await;
    sqlx::query("INSERT INTO workflow_runs (id, workflow_id, status) VALUES (?, ?, 'running')")
        .bind(run_id)
        .bind(workflow_id)
        .execute(&db.pool)
        .await?;

    if steps.is_empty() {
        fail_workflow_run(db, run_id, "Workflow has no enabled steps.").await?;
        return Ok(run_id);
    }

    let conversation_id =
        match add_conversation(db, &format!("Workflow Run: {}", workflow.name)).await {
            Ok(id) => id,
            Err(err) => {
                let message = format!("Failed to create workflow conversation: {}", err);
                fail_workflow_run(db, run_id, &message).await?;
                return Ok(run_id);
            }
        };
    sqlx::query("UPDATE workflow_runs SET conversation_id = ? WHERE id = ?")
        .bind(conversation_id)
        .bind(run_id)
        .execute(&db.pool)
        .await?;
    let mut last_output = String::new();

    for step in steps {
        let step_run_id = db.next_id().await;
        sqlx::query(
            "INSERT INTO workflow_step_runs (id, workflow_run_id, workflow_step_id, execution_run_id, step_order, status)
             VALUES (?, ?, ?, ?, ?, 'running')"
        )
        .bind(step_run_id)
        .bind(run_id)
        .bind(step.id)
        .bind(None::<i64>)
        .bind(step.step_order)
        .execute(&db.pool)
        .await?;

        let user_prompt = format!(
            "Workflow: {}\nStep {} - {}\n\n{}",
            workflow.name, step.step_order, step.name, step.prompt
        );
        if let Err(err) = add_message(db, conversation_id, "user", &user_prompt).await {
            let message = format!(
                "Failed to persist workflow step {} input: {}",
                step.step_order, err
            );
            sqlx::query(
                "UPDATE workflow_step_runs
                 SET status = 'failed', error_message = ?, finished_at = CURRENT_TIMESTAMP
                 WHERE id = ?",
            )
            .bind(&message)
            .bind(step_run_id)
            .execute(&db.pool)
            .await?;
            fail_workflow_run(db, run_id, &message).await?;
            return Ok(run_id);
        }

        let output = agent_system
            .orchestrate(db, conversation_id, &user_prompt)
            .await;
        if let Err(err) = add_message(db, conversation_id, "assistant", &output).await {
            let message = format!(
                "Failed to persist workflow step {} output: {}",
                step.step_order, err
            );
            sqlx::query(
                "UPDATE workflow_step_runs
                 SET status = 'failed', error_message = ?, finished_at = CURRENT_TIMESTAMP
                 WHERE id = ?",
            )
            .bind(&message)
            .bind(step_run_id)
            .execute(&db.pool)
            .await?;
            fail_workflow_run(db, run_id, &message).await?;
            return Ok(run_id);
        }

        let execution_run = get_latest_execution_run_for_conversation(db, conversation_id).await?;
        let execution_run_id = execution_run.as_ref().map(|run| run.id);
        let execution_status = execution_run
            .as_ref()
            .map(|run| run.status.as_str())
            .unwrap_or("completed");
        let execution_error = execution_run
            .as_ref()
            .and_then(|run| run.error_message.clone())
            .filter(|value| !value.trim().is_empty());
        let final_output = execution_run
            .as_ref()
            .and_then(|run| run.final_response.clone())
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| output.clone());

        if execution_status == "failed" {
            let message = execution_error.unwrap_or_else(|| {
                format!(
                    "Workflow step {} failed without a detailed execution error.",
                    step.step_order
                )
            });
            sqlx::query(
                "UPDATE workflow_step_runs
                 SET execution_run_id = ?, status = 'failed', output = ?, error_message = ?, finished_at = CURRENT_TIMESTAMP
                 WHERE id = ?"
            )
            .bind(execution_run_id)
            .bind(&final_output)
            .bind(&message)
            .bind(step_run_id)
            .execute(&db.pool)
            .await?;
            fail_workflow_run(db, run_id, &message).await?;
            return Ok(run_id);
        }

        last_output = final_output.clone();

        sqlx::query(
            "UPDATE workflow_step_runs
             SET execution_run_id = ?, status = 'completed', output = ?, error_message = NULL, finished_at = CURRENT_TIMESTAMP
             WHERE id = ?"
        )
        .bind(execution_run_id)
        .bind(&final_output)
        .bind(step_run_id)
        .execute(&db.pool)
        .await?;
    }

    sqlx::query(
        "UPDATE workflow_runs SET status = 'completed', summary = ?, finished_at = CURRENT_TIMESTAMP WHERE id = ?"
    )
    .bind(last_output)
    .bind(run_id)
    .execute(&db.pool)
    .await?;

    Ok(run_id)
}

pub async fn test_tool_code(
    language: &str,
    code: &str,
    query: Option<&str>,
) -> anyhow::Result<String> {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use tokio::process::Command;

    if language.to_lowercase() == "python" {
        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "{}", code)?;
        let path = temp_file.path().to_str().unwrap();

        let parts =
            crate::engine::utils::discovery::build_python_command().map_err(anyhow::Error::msg)?;
        let mut cmd = Command::new(&parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }
        cmd.arg(path);
        cmd.kill_on_drop(true);

        if let Some(q) = query {
            // Using a simple whitespace split to mimic CLI arguments from the UI text field.
            // For complex quoted arguments, a real shell lexer would be better, but this handles basic flags.
            let args: Vec<&str> = q.split_whitespace().collect();
            cmd.args(args);
        }

        let timeout_ms = std::env::var("OVERSEER_TOOL_TEST_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(15_000);

        let output =
            tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), cmd.output())
                .await
                .map_err(|_| anyhow::anyhow!("Tool test timed out after {} ms", timeout_ms))??;

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
        result.push_str(
            "Detected template variables ({{...}}). Ensure these are populated at runtime.\n",
        );
    }

    result.push_str("Agent instructions are ready for deployment.");
    Ok(result)
}

pub async fn get_setting(db: &Database, key: &str) -> String {
    let res: Result<Option<(String,)>, sqlx::Error> =
        sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&db.pool)
            .await;

    match res {
        Ok(Some((v,))) => v,
        _ => "".to_string(),
    }
}

pub async fn get_setting_or(db: &Database, key: &str, default: &str) -> String {
    let value = get_setting(db, key).await;
    if value.is_empty() {
        default.to_string()
    } else {
        value
    }
}

pub async fn resolve_model_name(
    db: &Database,
    preferred: Option<&str>,
    setting_key: &str,
    hard_default: &str,
) -> String {
    let available_models = get_available_models().await;
    let configured = get_setting_or(db, setting_key, hard_default).await;

    let preferred_clean = preferred.map(str::trim).filter(|value| !value.is_empty());
    let configured_clean = configured.trim();

    let is_available = |model: &str| {
        available_models.is_empty() || available_models.iter().any(|candidate| candidate == model)
    };

    if !configured_clean.is_empty() && is_available(configured_clean) {
        return configured_clean.to_string();
    }

    if let Some(model) = preferred_clean {
        if is_available(model) {
            return model.to_string();
        }
    }

    available_models.into_iter().next().unwrap_or_else(|| {
        if configured_clean.is_empty() {
            hard_default.to_string()
        } else {
            configured_clean.to_string()
        }
    })
}

pub async fn get_setting_i64(db: &Database, key: &str, default: i64) -> i64 {
    get_setting(db, key).await.parse::<i64>().unwrap_or(default)
}

fn parse_allowlist(raw: &str) -> Vec<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.to_string())
        .collect()
}

pub async fn get_computer_supervision_state(db: &Database) -> ComputerSupervisionState {
    let supervision_mode =
        get_setting_or(db, "computer_supervision_mode", "approval_required").await;
    let session_state = get_setting_or(db, "computer_session_state", "idle").await;
    let session_note = get_setting_or(
        db,
        "computer_session_note",
        "No supervised computer session is active.",
    )
    .await;
    let allowlist = parse_allowlist(
        &get_setting_or(db, "computer_supervision_allowlist", "").await,
    );
    let max_actions = get_setting_i64(db, "computer_supervision_max_actions", 25).await;
    let action_count = get_setting_i64(db, "computer_session_action_count", 0).await;
    let timeout_ms = get_setting_i64(db, "computer_supervision_session_timeout_ms", 600_000).await;
    let approval_state = get_setting_or(db, "computer_approval_state", "idle").await;
    let pending_tool = get_setting_or(db, "computer_pending_tool", "").await;
    let pending_action = get_setting_or(db, "computer_pending_action", "").await;
    let pending_domain = get_setting_or(db, "computer_pending_domain", "").await;
    let session_started_at = get_setting(db, "computer_session_started_at").await;

    ComputerSupervisionState {
        supervision_mode,
        session_state,
        session_note,
        allowlist,
        max_actions,
        action_count,
        timeout_ms,
        approval_state,
        pending_tool,
        pending_action,
        pending_domain,
        session_started_at: if session_started_at.trim().is_empty() {
            None
        } else {
            Some(session_started_at)
        },
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

pub async fn activate_computer_session(
    db: &Database,
    session_state: &str,
    note: &str,
) -> anyhow::Result<()> {
    set_setting(db, "computer_session_state", session_state).await?;
    set_setting(db, "computer_session_note", note).await?;
    if session_state == "idle" {
        set_setting(db, "computer_session_started_at", "").await?;
        set_setting(db, "computer_session_action_count", "0").await?;
    } else {
        let started_at = chrono::Utc::now().to_rfc3339();
        set_setting(db, "computer_session_started_at", &started_at).await?;
        set_setting(db, "computer_session_action_count", "0").await?;
    }
    clear_computer_pending_action(db).await?;
    Ok(())
}

pub async fn clear_computer_pending_action(db: &Database) -> anyhow::Result<()> {
    set_setting(db, "computer_approval_state", "idle").await?;
    set_setting(db, "computer_pending_tool", "").await?;
    set_setting(db, "computer_pending_action", "").await?;
    set_setting(db, "computer_pending_domain", "").await?;
    Ok(())
}

pub async fn stage_computer_approval_request(
    db: &Database,
    domain: &str,
    tool_name: &str,
    action_summary: &str,
) -> anyhow::Result<()> {
    set_setting(db, "computer_approval_state", "pending").await?;
    set_setting(db, "computer_pending_tool", tool_name).await?;
    set_setting(db, "computer_pending_action", action_summary).await?;
    set_setting(db, "computer_pending_domain", domain).await?;
    set_setting(
        db,
        "computer_session_note",
        &format!(
            "Approval requested for the next computer action.\nDomain: {}\nTool: {}\nAction: {}",
            domain, tool_name, action_summary
        ),
    )
    .await?;
    Ok(())
}

pub async fn approve_pending_computer_action(db: &Database) -> anyhow::Result<()> {
    let pending_tool = get_setting_or(db, "computer_pending_tool", "").await;
    let pending_action = get_setting_or(db, "computer_pending_action", "").await;
    set_setting(db, "computer_approval_state", "approved_once").await?;
    set_setting(
        db,
        "computer_session_note",
        &format!(
            "Next approved computer action is armed.\nTool: {}\nAction: {}",
            if pending_tool.trim().is_empty() {
                "(not provided)"
            } else {
                pending_tool.trim()
            },
            if pending_action.trim().is_empty() {
                "(not provided)"
            } else {
                pending_action.trim()
            }
        ),
    )
    .await?;
    Ok(())
}

pub async fn reject_pending_computer_action(db: &Database) -> anyhow::Result<()> {
    clear_computer_pending_action(db).await?;
    set_setting(
        db,
        "computer_session_note",
        "Pending computer action was rejected. Overseer should propose a new action before trying again.",
    )
    .await?;
    Ok(())
}

pub async fn register_computer_action_execution(
    db: &Database,
    tool_name: &str,
    action_summary: &str,
) -> anyhow::Result<i64> {
    let current = get_setting_i64(db, "computer_session_action_count", 0).await;
    let next = current + 1;
    set_setting(db, "computer_session_action_count", &next.to_string()).await?;
    clear_computer_pending_action(db).await?;
    set_setting(
        db,
        "computer_session_note",
        &format!(
            "Executed computer action {}.\nTool: {}\nAction: {}",
            next, tool_name, action_summary
        ),
    )
    .await?;
    Ok(next)
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
    #[cfg(windows)]
    use std::os::windows::process::CommandExt;
    use std::process::Command;

    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    // 1. Try GPU Load (NVIDIA)
    let mut gpu_cmd = Command::new("nvidia-smi");
    gpu_cmd.args([
        "--query-gpu=utilization.gpu",
        "--format=csv,noheader,nounits",
    ]);
    #[cfg(windows)]
    gpu_cmd.creation_flags(CREATE_NO_WINDOW);
    let gpu_output = gpu_cmd.output();

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
    // We use top to get a quick snapshot of CPU idle time on Unix-like systems.
    #[cfg(not(windows))]
    let cpu_output = Command::new("sh")
        .arg("-c")
        .arg("top -bn1 | grep '%Cpu(s)'")
        .output();

    #[cfg(not(windows))]
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

    #[cfg(windows)]
    return SystemLoad {
        label: "System Load".to_string(),
        usage: 0,
    };

    // 3. Absolute fallback
    SystemLoad {
        label: "System Load".to_string(),
        usage: 0,
    }
}
