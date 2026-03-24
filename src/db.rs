// Database implementation for Overseer
// Triggering re-build for migrations (v4 - final precision fix)
use anyhow::Context;
use serde::{Deserialize, Serialize};
use snowflake::SnowflakeIdGenerator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{ConnectOptions, Connection};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Database {
    pub pool: sqlx::SqlitePool,
    pub snowflake: Arc<Mutex<SnowflakeIdGenerator>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Supervisor {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub model: Option<String>,
    pub domain: String,
    pub version: Option<String>,
    pub code: Option<String>,
    #[sqlx(default)]
    pub bypass_synthesizer: bool,
    pub fast_path_keywords: Option<String>,
    pub verification_agent_id: Option<i64>,
    pub routing_priority: i32,
    pub hide_evidence: bool,
    pub report_style: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct SupervisorAgent {
    pub id: i64,
    pub supervisor_id: i64,
    pub agent_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Planner {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub model: Option<String>,
    pub domain: String,
    pub version: Option<String>,
    pub code: Option<String>,
}

impl PartialEq for Database {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Database {
    pub async fn new() -> anyhow::Result<Self> {
        let db_url = "sqlite:overseer.db?mode=rwc";
        println!("🚀 Connecting to SQLite at {}...", db_url);

        let mut connection_options = SqliteConnectOptions::from_str(db_url)?
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(30)); // Longer timeout

        connection_options = connection_options
            .disable_statement_logging()
            .log_statements(log::LevelFilter::Off)
            .log_slow_statements(log::LevelFilter::Warn, std::time::Duration::from_secs(30));

        println!("📜 Running database migrations (Internal Connection)...");
        let pool = SqlitePoolOptions::new()
            .max_connections(1) // Single connection for SQLite is often faster and avoids locks
            .acquire_timeout(std::time::Duration::from_secs(30))
            .acquire_slow_threshold(std::time::Duration::from_secs(30))
            .acquire_slow_level(log::LevelFilter::Off)
            .connect_with(connection_options)
            .await
            .context("failed to connect to SQLite database")?;

        // Synchronous = OFF is safe during migrations for speed, but keep journal_mode = WAL
        sqlx::query("PRAGMA synchronous = OFF")
            .execute(&pool)
            .await
            .context("failed to relax SQLite synchronous mode for migrations")?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .context("failed while applying database migrations")?;

        // Restore durability
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await
            .context("failed to restore SQLite synchronous mode after migrations")?;

        // Pool is already created above and migrated

        println!("📂 Initializing LanceDB...");
        let generator = SnowflakeIdGenerator::new(1, 1);
        let database = Self {
            pool: pool.clone(),
            snowflake: Arc::new(Mutex::new(generator)),
        };

        database
            .seed_all()
            .await
            .context("failed to seed or normalize database content")?;

        println!("🏗️  Staging tools to runtime environment...");
        if let Err(e) = crate::engine::utils::discovery::prepare_runtime_env(&database).await {
            println!("⚠️  Tool runtime staging failed: {}", e);
        }

        println!("✨ Database initialization complete.");
        Ok(database)
    }

    async fn seed_all(&self) -> anyhow::Result<()> {
        println!("🌱 Checking database seeding state...");
        self.seed_agents().await?;
        self.seed_tools().await?;
        self.seed_agent_tools().await?;
        self.seed_supervisors().await?;
        self.seed_planners().await?;
        self.ensure_setting_defaults().await?;
        self.ensure_native_core_tools().await?;
        self.ensure_system_agents().await?;
        self.ensure_system_agent_tools().await?;
        self.ensure_system_supervisors().await?;
        self.backfill_ids().await?;
        Ok(())
    }

    async fn ensure_setting_defaults(&self) -> anyhow::Result<()> {
        let default_model = crate::backend::get_available_models()
            .await
            .into_iter()
            .next()
            .unwrap_or_else(|| "qwen2.5:7b-instruct".to_string());
        let defaults = [
            ("title_generator_model", default_model.as_str()),
            ("default_chat_model", default_model.as_str()),
            ("max_iterations", "10"),
            ("tool_timeout_ms", "45000"),
            ("model_timeout_ms", "90000"),
            ("computer_supervision_mode", "approval_required"),
            ("computer_supervision_max_actions", "25"),
            ("computer_supervision_session_timeout_ms", "600000"),
            ("computer_supervision_allowlist", ""),
            ("computer_session_state", "idle"),
            ("computer_session_note", "No supervised computer session is active."),
            ("computer_approval_state", "idle"),
            ("computer_pending_tool", ""),
            ("computer_pending_action", ""),
            ("computer_pending_domain", ""),
            ("computer_session_action_count", "0"),
            ("computer_session_started_at", ""),
        ];

        for (key, value) in defaults {
            let id = self.next_id().await;
            sqlx::query(
                "INSERT INTO settings (id, key, value) VALUES (?, ?, ?) \
                 ON CONFLICT(key) DO NOTHING",
            )
            .bind(id)
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn ensure_system_agents(&self) -> anyhow::Result<()> {
        // Upserts agents that must always exist regardless of seeding state.
        // Safe to run every startup — uses INSERT OR IGNORE so no duplicates.
        let system_agents: Vec<(&str, &str, &str, &str, &str)> = vec![
            (
                "Synthesizer",
                "Synthesizes research findings into clean user-friendly responses",
                "synthesizer",
                "qwen2.5:7b-instruct",
                "You are a helpful assistant. Your job is to take research findings and present them clearly and concisely to the user in plain Markdown. Structure your response with headers and bullet points where appropriate. Never output raw JSON arrays or technical plan data. Always write as if speaking directly to the user.",
            ),
            (
                "Computer Operator",
                "Operates a supervised computer session with explicit safety boundaries, approvals, and evidence gathering.",
                "computer",
                "qwen2.5:7b-instruct",
                "You are the Computer Operator for Overseer. You may observe, plan, and act on the user's computer only within approved safety policies. Prefer describing the next safe action before acting. Record evidence, mention the target window or app, and stop when confidence is low or a blocked action is encountered.",
            ),
        ];

        for (name, desc, domain, model, inst) in system_agents {
            let exists: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents WHERE domain = ?")
                .bind(domain)
                .fetch_one(&self.pool)
                .await?;

            if exists.0 == 0 {
                let id = self.next_id().await;
                sqlx::query(
                "INSERT OR IGNORE INTO agents (id, name, description, domain, model, instructions, version, code, creator_id) 
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(name)
            .bind(desc)
            .bind(domain)
            .bind(model)
            .bind(inst)
            .bind("1.0.0")
            .bind(None::<String>)
            .bind(None::<i64>)  // creator_id is null
            .execute(&self.pool)
            .await?;

                println!("✅ Inserted missing system agent: {} ({})", name, domain);
            } else {
                println!("✓ System agent '{}' already exists, skipping.", domain);
            }
        }

        Ok(())
    }

    async fn ensure_native_core_tools(&self) -> anyhow::Result<()> {
        let native_tools = [
            (
                "stealth_search",
                "Native search tool that returns result titles, URLs, and snippets.",
            ),
            (
                "stealth_browse",
                "Native browser fetch tool that returns cleaned page text and metadata.",
            ),
            (
                "http_request",
                "Native HTTP GET/POST request tool for direct endpoint checks.",
            ),
            (
                "app_launcher",
                "Native application launcher for local desktop tasks.",
            ),
        ];

        for (name, description) in native_tools {
            sqlx::query(
                "UPDATE tools
                 SET language = 'rust',
                     script_path = NULL,
                     description = COALESCE(NULLIF(description, ''), ?),
                     code = '// Native implementation'
                 WHERE name = ?",
            )
            .bind(description)
            .bind(name)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn ensure_system_supervisors(&self) -> anyhow::Result<()> {
        let system_supervisors: Vec<(&str, &str, &str, &str, &str)> = vec![(
            "Computer Supervisor",
            "Supervises broader computer-operation sessions with approvals, safety checkpoints, and evidence capture",
            "computer",
            "qwen2.5:7b-instruct",
            "You are the Computer Supervisor for Overseer. You control broad computer-operation requests only through supervised execution. Respect the configured supervision mode, ask for approval when required, keep actions bounded, and never claim success without evidence from the desktop state or tool results.",
        )];

        for (name, desc, domain, model, instructions) in system_supervisors {
            let existing: Option<(i64,)> =
                sqlx::query_as("SELECT id FROM supervisors WHERE domain = ? LIMIT 1")
                    .bind(domain)
                    .fetch_optional(&self.pool)
                    .await?;

            let supervisor_id = if let Some((id,)) = existing {
                id
            } else {
                let id = self.next_id().await;
                sqlx::query(
                    "INSERT INTO supervisors (id, name, description, instructions, model, domain, version, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(id)
                .bind(name)
                .bind(desc)
                .bind(instructions)
                .bind(model)
                .bind(domain)
                .bind("1.0.0")
                .bind(None::<String>)
                .execute(&self.pool)
                .await?;
                println!("✅ Inserted missing system supervisor: {} ({})", name, domain);
                id
            };

            if let Some((agent_id,)) = sqlx::query_as::<_, (i64,)>(
                "SELECT id FROM agents WHERE domain = ? LIMIT 1",
            )
            .bind(domain)
            .fetch_optional(&self.pool)
            .await?
            {
                let relation_id = self.next_id().await;
                sqlx::query(
                    "INSERT OR IGNORE INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (?, ?, ?)",
                )
                .bind(relation_id)
                .bind(supervisor_id)
                .bind(agent_id)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn ensure_system_agent_tools(&self) -> anyhow::Result<()> {
        let mappings = [
            ("computer", "os_tool"),
            ("computer", "vision_analysis"),
            ("computer", "charting"),
        ];

        for (agent_domain, tool_name) in mappings {
            let agent = sqlx::query_as::<_, (i64,)>("SELECT id FROM agents WHERE domain = ? LIMIT 1")
                .bind(agent_domain)
                .fetch_optional(&self.pool)
                .await?;
            let tool = sqlx::query_as::<_, (i64,)>("SELECT id FROM tools WHERE name = ? LIMIT 1")
                .bind(tool_name)
                .fetch_optional(&self.pool)
                .await?;

            if let (Some((agent_id,)), Some((tool_id,))) = (agent, tool) {
                let relation_id = self.next_id().await;
                sqlx::query(
                    "INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id) VALUES (?, ?, ?)",
                )
                .bind(relation_id)
                .bind(agent_id)
                .bind(tool_id)
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn seed_agents(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            return Ok(());
        }

        println!("🌱 Seeding authentic agents from configs and codebase...");

        let mut agents_data = Vec::new();

        // 1. Try to load from config/agents/*.json
        if let Some(config_dir) = crate::engine::utils::discovery::config_agents_dir() {
            if let Ok(entries) = std::fs::read_dir(config_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                        if let Ok(content) = std::fs::read_to_string(entry.path()) {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                                agents_data.push((
                                    val["name"].as_str().unwrap_or("Unknown").to_string(),
                                    val["description"].as_str().unwrap_or("").to_string(),
                                    val["domain"].as_str().unwrap_or("core").to_string(),
                                    val["model"]
                                        .as_str()
                                        .unwrap_or("qwen2.5:7b-instruct")
                                        .to_string(),
                                    val["instructions"].as_str().unwrap_or("").to_string(),
                                    val["version"].as_str().unwrap_or("1.0.0").to_string(),
                                    val["code"].as_str().map(|s| s.to_string()),
                                ));
                            }
                        }
                    }
                }
            }
        }

        // 2. Add others not in configs with real preambles
        let codebase_agents = vec![
            (
                "Master Orchestrator",
                "Primary system planner",
                "orchestrator",
                "qwen2.5:7b-instruct",
                "src/engine/agents/master_planner/preamble.md",
            ),
            (
                "Overseer Agent",
                "Task verification and grounding supervisor",
                "overseer",
                "qwen2.5:7b-instruct",
                "",
            ),
            (
                "Web Planner",
                "Web task planner",
                "web_planner",
                "llama3",
                "src/engine/agents/web/planner_preamble.md",
            ),
            (
                "Office Planner",
                "Office task planner",
                "office_planner",
                "qwen2.5:7b-instruct",
                "",
            ),
            (
                "Desktop Planner",
                "Desktop task planner",
                "desktop_planner",
                "qwen2.5:7b-instruct",
                "",
            ),
            (
                "Neontune Planner",
                "Music task planner",
                "neontune_planner",
                "qwen2.5:7b-instruct",
                "",
            ),
            (
                "Chat Assistant",
                "General chat and help",
                "chat",
                "llama3",
                "src/engine/agents/chat/preamble.md",
            ),
            (
                "Vision Assistant",
                "Image analysis specialist",
                "vision",
                "llava",
                "",
            ),
        ];

        for (name, desc, domain, model, path) in codebase_agents {
            if !agents_data.iter().any(|a| a.0 == name) {
                let inst = if path.is_empty() && domain == "vision" {
                    "You are a vision-capable agent. Describe the current screen state accurately. ALWAYS use the vision_analysis tool for images.".to_string()
                } else if path.is_empty() && domain == "overseer" {
                    "You are the Overseer. Your job is to verify if the user's request has been fully completed based on the grounding and plan history. Respond with YES or NO: [reason].".to_string()
                } else if !path.is_empty() {
                    std::fs::read_to_string(path).unwrap_or_else(|_| "Default preamble".to_string())
                } else {
                    "Default preamble".to_string()
                };
                agents_data.push((
                    name.to_string(),
                    desc.to_string(),
                    domain.to_string(),
                    model.to_string(),
                    inst,
                    "1.0.0".to_string(),
                    None::<String>,
                ));
            }
        }

        let mut tx = self.pool.begin().await?;
        for (name, desc, domain, model, inst, version, code) in agents_data {
            let id = self.next_id().await;
            sqlx::query("INSERT INTO agents (id, name, description, domain, model, instructions, version, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(id).bind(name).bind(desc).bind(domain).bind(model).bind(inst).bind(version).bind(code)
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn seed_tools(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tools")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            return Ok(());
        }

        println!("🛠️ Seeding ALL authentic tools from filesystem...");
        let mut tools_data = Vec::new();

        let script_dirs = ["web", "stealth", "desktop"];
        let script_roots = crate::engine::utils::discovery::candidate_tool_script_roots();
        for root in script_roots {
            for dir in script_dirs {
                let path = root.join(dir);
                if let Ok(entries) = std::fs::read_dir(&path) {
                    for entry in entries.flatten() {
                        if entry.path().extension().and_then(|s| s.to_str()) == Some("py") {
                            if let Some(name) = entry.file_name().to_str() {
                                let tool_name = name.trim_end_matches(".py").to_string();
                                if tools_data.iter().any(
                                    |existing: &(String, String, String, String, String)| {
                                        existing.0.as_str() == tool_name.as_str()
                                    },
                                ) {
                                    continue;
                                }
                                let script_path = format!(
                                    "{}/{}/{}",
                                    crate::engine::utils::discovery::logical_script_root(&root),
                                    dir,
                                    name
                                );
                                let code =
                                    std::fs::read_to_string(entry.path()).unwrap_or_default();
                                let description = format!("{} tool for {} domain", tool_name, dir);
                                tools_data.push((
                                    tool_name,
                                    description,
                                    "python".to_string(),
                                    script_path,
                                    code,
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Add native tools
        tools_data.push((
            "os_tool".to_string(),
            "Native system access".to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "vision_analysis".to_string(),
            "Analyze images and perform OCR. Supports URLs and local paths.".to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "charting".to_string(),
            "Create beautiful charts (bar, line, pie) from data. Returns SVG.".to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));

        let mut tx = self.pool.begin().await?;
        for (name, desc, lang, path, code) in tools_data {
            let id = self.next_id().await;
            let script_path = if path.is_empty() { None } else { Some(path) };
            sqlx::query("INSERT INTO tools (id, name, description, language, script_path, code, version) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind(id).bind(name).bind(desc).bind(lang).bind(script_path).bind(code).bind("1.0.0")
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn seed_agent_tools(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agent_tools")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            return Ok(());
        }

        println!("🔗 Linking agents and tools with comprehensive mappings...");
        let mut tx = self.pool.begin().await?;

        let mappings = vec![
            ("Web Planner", "browser_automation"),
            ("Web Supervisor", "browser_automation"),
            ("Stealth Supervisor", "stealth_search"),
            ("Desktop Supervisor", "ui_automation"),
            ("Master Orchestrator", "os_tool"),
            ("Master Orchestrator", "ui_automation"),
            ("Master Orchestrator", "browser_automation"),
            ("Chat Supervisor", "charting"),
        ];

        for (agent_name, tool_name) in mappings {
            let agent: Option<(i64,)> =
                sqlx::query_as("SELECT id FROM agents WHERE name = ? LIMIT 1")
                    .bind(agent_name)
                    .fetch_optional(&mut *tx)
                    .await?;
            let tool: Option<(i64,)> =
                sqlx::query_as("SELECT id FROM tools WHERE name = ? LIMIT 1")
                    .bind(tool_name)
                    .fetch_optional(&mut *tx)
                    .await?;

            if let (Some(a), Some(t)) = (agent, tool) {
                let id = self.next_id().await;
                sqlx::query(
                    "INSERT OR IGNORE INTO agent_tools (id, agent_id, tool_id) VALUES (?, ?, ?)",
                )
                .bind(id)
                .bind(a.0)
                .bind(t.0)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn seed_supervisors(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM supervisors")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            return Ok(());
        }

        println!("🛸 Seeding dynamic supervisors...");
        let supervisors = vec![
            (
                "Web Supervisor",
                "Orchestrates web research and automation",
                "web",
                "llama3",
                "src/engine/agents/web/planner_preamble.md",
            ),
            (
                "Office Supervisor",
                "Manages document and productivity tasks",
                "office",
                "qwen2.5:7b-instruct",
                "",
            ),
            (
                "Desktop Supervisor",
                "Controls native desktop applications",
                "desktop",
                "qwen2.5:7b-instruct",
                "",
            ),
            (
                "Stealth Supervisor",
                "Executes sensitive background research",
                "stealth",
                "qwen2.5:7b-instruct",
                "",
            ),
            (
                "Neontune Supervisor",
                "Specialized music and media metadata manager",
                "neontune",
                "qwen2.5:7b-instruct",
                "",
            ),
            (
                "Text Writer Supervisor",
                "High-quality content generation and editing",
                "text_writer",
                "qwen2.5:7b-instruct",
                "",
            ),
            (
                "Chat Supervisor",
                "General conversation manager",
                "chat",
                "llama3",
                "",
            ),
            (
                "Vision Supervisor",
                "Visual perception manager",
                "vision",
                "qwen2.5:7b-instruct",
                "",
            ),
        ];

        let mut tx = self.pool.begin().await?;
        for (name, desc, domain, model, path) in supervisors {
            let id = self.next_id().await;
            let inst = if !path.is_empty() {
                std::fs::read_to_string(path).unwrap_or_else(|_| format!("You are the {} supervisor. Your mission is to complete the user request using the available tools via the `execute_tool` router. Provide a comprehensive final answer after tool usage.", name))
            } else {
                format!("You are the {} supervisor. Your mission is to complete the user request using the available tools via the `execute_tool` router. Provide a comprehensive final answer after tool usage.", name)
            };

            sqlx::query("INSERT INTO supervisors (id, name, description, instructions, model, domain, version, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(id).bind(name).bind(desc).bind(inst).bind(model).bind(domain).bind("1.0.0").bind(None::<String>)
                .execute(&mut *tx).await?;

            // Auto-link agents based on domain
            let agent_domain = match domain {
                "web" => Some("web"),
                "stealth" => Some("stealth"),
                "desktop" => Some("desktop"),
                "office" => Some("office"),
                "neontune" => Some("neontune"),
                "text_writer" => Some("text_writer"),
                "chat" => Some("chat"),
                "vision" => Some("vision"),
                _ => None,
            };

            if let Some(d) = agent_domain {
                let agents: Vec<(i64,)> =
                    sqlx::query_as("SELECT id FROM agents WHERE domain = ? OR domain LIKE ?")
                        .bind(d)
                        .bind(format!("{}%", d))
                        .fetch_all(&mut *tx)
                        .await?;

                for (a_id,) in agents {
                    let rel_id = self.next_id().await;
                    sqlx::query("INSERT INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (?, ?, ?)")
                        .bind(rel_id).bind(id).bind(a_id).execute(&mut *tx).await?;
                }
            }
        }
        tx.commit().await?;
        Ok(())
    }

    async fn seed_planners(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM planners")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            return Ok(());
        }

        println!("🌱 Seeding Planners...");
        let planners = vec![
            (
                "Web Planner",
                "Plans browser-based tasks step-by-step.",
                "web",
                "web_planner",
            ),
            (
                "Office Planner",
                "Plans office document tasks (Writer, Calc, etc).",
                "office",
                "office_planner",
            ),
            (
                "Desktop Planner",
                "Plans desktop and application-level tasks.",
                "desktop",
                "desktop_planner",
            ),
            (
                "Neontune Planner",
                "Plans music and audio library operations.",
                "neontune",
                "neontune_planner",
            ),
        ];

        for (name, desc, domain, domain_id) in planners {
            let id = self.next_id().await;
            sqlx::query(
                "INSERT INTO planners (id, name, description, instructions, domain, model, version, code) VALUES (?, ?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(name)
            .bind(desc)
            .bind(format!("Plan {} tasks step by step. Use the domain '{}'.", domain, domain))
            .bind(domain_id)
            .bind("qwen2.5:7b-instruct")
            .bind("1.0.0")
            .bind("")
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    async fn backfill_ids(&self) -> anyhow::Result<()> {
        let mut conn = self.pool.acquire().await?;

        // Disable foreign keys temporarily for ID migration
        sqlx::query("PRAGMA foreign_keys = OFF")
            .execute(&mut *conn)
            .await?;

        let mut tx = conn.begin().await?;

        println!("🔄 Backfilling Missing Snowflake IDs...");

        // 1. Backfill settings
        let settings_to_backfill: Vec<(String,)> =
            sqlx::query_as("SELECT key FROM settings WHERE id IS NULL")
                .fetch_all(&mut *tx)
                .await?;
        for (key,) in settings_to_backfill {
            let id = self.next_id().await;
            sqlx::query("UPDATE settings SET id = ? WHERE key = ?")
                .bind(id)
                .bind(key)
                .execute(&mut *tx)
                .await?;
        }

        // 2. Backfill agent_tools
        let agent_tools_to_backfill: Vec<(i64, i64)> =
            sqlx::query_as("SELECT agent_id, tool_id FROM agent_tools WHERE id IS NULL")
                .fetch_all(&mut *tx)
                .await?;
        for (a_id, t_id) in agent_tools_to_backfill {
            let id = self.next_id().await;
            sqlx::query("UPDATE agent_tools SET id = ? WHERE agent_id = ? AND tool_id = ?")
                .bind(id)
                .bind(a_id)
                .bind(t_id)
                .execute(&mut *tx)
                .await?;
        }

        // 3. Backfill agents with sequential IDs (and update foreign keys)
        let agents_to_backfill: Vec<(i64,)> =
            sqlx::query_as("SELECT id FROM agents WHERE id < 1000000000000000000")
                .fetch_all(&mut *tx)
                .await?;

        for (old_id,) in agents_to_backfill {
            let new_id = self.next_id().await;

            // Update foreign keys in agent_tools
            sqlx::query("UPDATE agent_tools SET agent_id = ? WHERE agent_id = ?")
                .bind(new_id)
                .bind(old_id)
                .execute(&mut *tx)
                .await?;

            // Update main agent ID
            sqlx::query("UPDATE agents SET id = ? WHERE id = ?")
                .bind(new_id)
                .bind(old_id)
                .execute(&mut *tx)
                .await?;

            println!("✅ Migrated agent ID {} -> {}", old_id, new_id);
        }

        tx.commit().await?;

        // Re-enable foreign keys
        sqlx::query("PRAGMA foreign_keys = ON")
            .execute(&mut *conn)
            .await?;
        Ok(())
    }

    pub async fn next_id(&self) -> i64 {
        let mut gen = self.snowflake.lock().await;
        gen.real_time_generate()
    }
}
