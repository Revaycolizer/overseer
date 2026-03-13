use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub tools: Vec<String>,
    pub instructions: String,
    pub model: String,
    pub domain: String,
    pub version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub code: String,
    pub language: String,
    pub script_path: Option<String>,
    pub version: String,
}

pub async fn get_agents(db: &crate::db::Database) -> Vec<AgentInfo> {
    crate::backend::get_agents(db).await.unwrap_or_default()
}

pub async fn get_tools(db: &crate::db::Database) -> Vec<ToolInfo> {
    crate::backend::get_tools(db).await.unwrap_or_default()
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
