use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

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
    pub safety_level: String,
    pub version: String,
}

pub async fn get_agents(db: &crate::db::Database) -> Vec<AgentInfo> {
    crate::backend::get_agents(db).await.unwrap_or_default()
}

pub async fn get_tools(db: &crate::db::Database) -> Vec<ToolInfo> {
    crate::backend::get_tools(db).await.unwrap_or_default()
}

pub fn project_root() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

pub fn runtime_dir() -> PathBuf {
    std::env::temp_dir().join("overseer_runtime")
}

fn normalize_script_path(raw_path: &str) -> PathBuf {
    let normalized = raw_path.replace('\\', "/");
    let rewritten = if let Some(rest) = normalized.strip_prefix("agents-scripts/") {
        format!("src/engine/scripts/{rest}")
    } else {
        normalized
    };

    let path = PathBuf::from(&rewritten);
    if path.is_absolute() {
        path
    } else {
        project_root().join(path)
    }
}

fn python_command_candidates() -> Vec<Vec<String>> {
    let mut candidates = Vec::new();

    if let Ok(explicit) = std::env::var("OVERSEER_PYTHON") {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            candidates.push(vec![trimmed.to_string()]);
        }
    }

    if cfg!(windows) {
        candidates.push(vec!["python".to_string()]);
        candidates.push(vec!["py".to_string(), "-3".to_string()]);
        candidates.push(vec!["py".to_string()]);
        candidates.push(vec!["python3".to_string()]);
    } else {
        candidates.push(vec!["python3".to_string()]);
        candidates.push(vec!["python".to_string()]);
    }

    candidates
}

pub fn detect_python_command() -> Option<Vec<String>> {
    for candidate in python_command_candidates() {
        let mut probe = Command::new(&candidate[0]);
        if candidate.len() > 1 {
            probe.args(&candidate[1..]);
        }
        probe.arg("--version");

        if let Ok(output) = probe.output() {
            if output.status.success() {
                return Some(candidate);
            }
        }
    }

    None
}

pub fn python_command_label() -> String {
    detect_python_command()
        .map(|parts| parts.join(" "))
        .unwrap_or_else(|| "python (not found)".to_string())
}

pub fn build_python_command() -> Result<Vec<String>, String> {
    detect_python_command()
        .ok_or_else(|| "No working Python interpreter found. Set OVERSEER_PYTHON or install Python 3.".to_string())
}

/// Prepares a dedicated runtime directory with all DB-backed Python tools.
/// This allows Python scripts to reference sibling files relatively.
pub async fn prepare_runtime_env(db: &crate::db::Database) -> anyhow::Result<String> {
    let runtime_dir = runtime_dir();
    std::fs::create_dir_all(&runtime_dir)?;

    let tools = get_tools(db).await;
    for tool in tools {
        if tool.language != "python" {
            continue;
        }

        let file_path = runtime_dir.join(format!("{}{}", tool.name, tool_extension(&tool.name)));

        let source = if !tool.code.trim().is_empty() {
            tool.code.clone()
        } else if let Some(script_path) = tool.script_path.as_deref() {
            let resolved = normalize_script_path(script_path);
            if resolved.exists() {
                std::fs::read_to_string(resolved).unwrap_or_default()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        if source.is_empty() {
            println!("⚠️  Skipping runtime staging for '{}' because no Python source was available.", tool.name);
            continue;
        }

        if let Err(e) = std::fs::write(&file_path, source) {
            println!("⚠️  Failed to stage tool '{}' to runtime: {}", tool.name, e);
        }
    }

    Ok(runtime_dir.to_string_lossy().to_string())
}

fn tool_extension(tool_name: &str) -> &'static str {
    if tool_name.contains('.') {
        ""
    } else {
        ".py"
    }
}

pub fn resolve_tool_script_path(tool: &ToolInfo) -> Option<PathBuf> {
    if tool.language != "python" {
        return tool.script_path.as_deref().map(normalize_script_path);
    }

    let staged = runtime_dir().join(format!("{}{}", tool.name, tool_extension(&tool.name)));
    if staged.exists() {
        return Some(staged);
    }

    tool.script_path
        .as_deref()
        .map(normalize_script_path)
        .filter(|path| path.exists())
}

/// Helper to get a path to an executable script for a tool.
pub fn get_tool_executable_path(tool: &ToolInfo) -> Option<String> {
    resolve_tool_script_path(tool).map(|path| path.to_string_lossy().to_string())
}

pub async fn print_system_info(db: &crate::db::Database) {
    let agents = get_agents(db).await;
    let tools = get_tools(db).await;

    println!("\n🚀 RIG AGENT SYSTEM CONFIGURATION");
    println!("=================================");
    println!("Total Agents: {}", agents.len());
    println!("Total Tools:  {}", tools.len());
    println!("Runtime Dir:  {}", runtime_dir().display());
    println!("Python Cmd:   {}", python_command_label());

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

    let mut tool_to_agents: HashMap<String, Vec<String>> = HashMap::new();
    for agent in &agents {
        for tool in &agent.tools {
            tool_to_agents
                .entry(tool.clone())
                .or_insert_with(Vec::new)
                .push(agent.name.clone());
        }
    }

    for tool in &tools {
        println!("⚙️  {:<24}", tool.name);
        println!("   Description: {}", tool.description);
        println!("   Safety Level: {}", tool.safety_level);
        if let Some(path) = resolve_tool_script_path(tool) {
            println!("   Runtime Path: {}", path.display());
        }
        if let Some(users) = tool_to_agents.get(&tool.name) {
            println!("   Utilized By: {}", users.join(", "));
        } else {
            println!("   Utilized By: (Infrastructure / Internal)");
        }
        println!();
    }
}
