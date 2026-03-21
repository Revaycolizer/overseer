use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow, PartialEq)]
pub struct AgentInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub instructions: String,
    pub model: String,
    pub domain: String,
    pub version: String,
    pub code: Option<String>,
    pub source_files: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, sqlx::FromRow, PartialEq)]
pub struct ToolInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub code: String,
    pub language: String,
    pub script_path: Option<String>,
    pub required_intent: Option<String>,
    pub version: String,
}

pub async fn get_agents(db: &crate::db::Database) -> Vec<AgentInfo> {
    crate::backend::get_agents(db).await.unwrap_or_default()
}

pub async fn get_tools(db: &crate::db::Database) -> Vec<ToolInfo> {
    crate::backend::get_tools(db).await.unwrap_or_default()
}

/// Prepares a dedicated directory in /tmp with all tools from the database.
/// This allows Python scripts to reference siblings (like browser_server.py) relatively.
pub async fn prepare_runtime_env(db: &crate::db::Database) -> anyhow::Result<String> {
    let runtime_dir = "/tmp/overseer_runtime";
    std::fs::create_dir_all(runtime_dir)?;

    let tools = get_tools(db).await;
    for tool in tools {
        if tool.language == "python" && !tool.code.is_empty() {
            let extension = if tool.name.contains('.') { "" } else { ".py" };
            let file_path = format!("{}/{}{}", runtime_dir, tool.name, extension);
            if let Err(e) = std::fs::write(&file_path, &tool.code) {
                println!("⚠️  Failed to stage tool '{}' to runtime: {}", tool.name, e);
            }
        }
    }
    Ok(runtime_dir.to_string())
}

/// Helper to get a path to an executable script for a tool.
/// Assumes prepare_runtime_env has been called.
pub fn get_tool_executable_path(tool: &ToolInfo) -> Option<String> {
    if tool.language == "python" {
        let extension = if tool.name.contains('.') { "" } else { ".py" };
        return Some(format!("/tmp/overseer_runtime/{}{}", tool.name, extension));
    }
    tool.script_path.clone()
}

pub async fn print_system_info(db: &crate::db::Database) {
    let agents = get_agents(db).await;
    let tools = get_tools(db).await;

    println!("\n🚀 RIG AGENT SYSTEM CONFIGURATION");
    println!("=================================");
    println!("Total Agents: {}", agents.len());
    println!("Total Tools:  {}", tools.len());

    println!("\n🤖 AVAILABLE AGENTS");
    println!("--------------------");
    for agent in &agents {
        println!("● {:<20} | {}", agent.name, agent.description);
        if !agent.tools.is_empty() {
            println!("  └── Equipped Tools: {}", agent.tools.join(", "));
        }
        println!();
    }

    println!("🛠️  SYSTEM TOOLS & USAGE");
    println!("-----------------------");
    
    // Build reverse mapping
    let mut tool_to_agents: HashMap<String, Vec<String>> = HashMap::new();
    for agent in &agents {
        // Assume tools is a JSON array string for now, but discovery might have it as Vec<String> if I restored it
        for tool in &agent.tools {
            tool_to_agents.entry(tool.clone()).or_insert_with(Vec::new).push(agent.name.clone());
        }
    }

    for tool in &tools {
        println!("⚙️  {:<24}", tool.name);
        println!("   Description: {}", tool.description);
        if let Some(users) = tool_to_agents.get(&tool.name) {
            println!("   Utilized By: {}", users.join(", "));
        } else {
            println!("   Utilized By: (Infrastructure / Internal)");
        }
        println!();
    }
}
