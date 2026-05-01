// Database implementation for Overseer
// Triggering re-build for migrations (v4 - final precision fix)
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha384};
use snowflake::SnowflakeIdGenerator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{ConnectOptions, Connection};
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
#[cfg(feature = "memory")]
use lancedb::connect;
#[cfg(feature = "memory")]
use arrow::array::{Float32Array, Int64Array, StringArray, RecordBatch};
#[cfg(feature = "memory")]
use arrow::datatypes::{DataType, Field, Schema};

#[derive(Clone)]
pub struct Database {
    pub pool: sqlx::SqlitePool,
    pub snowflake: Arc<Mutex<SnowflakeIdGenerator>>,
    pub lancedb_uri: String,
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
    #[sqlx(default)]
    pub skip_fast_path_if_artifacts: bool,
    pub artifact_fast_path_keywords: Option<String>,
    pub response_format: String,
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
    pub response_format: String,
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

        if cfg!(debug_assertions) {
            println!("🔍 Development mode detected: Using filesystem migrations from ./migrations");
            let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("./migrations"))
                .await
                .context("failed to create filesystem migrator for development")?;
            
            if let Err(error) = migrator.run(&pool).await {
                if try_reconcile_known_migration_checksum(&pool, &error)
                    .await
                    .context("failed while attempting to reconcile known migration checksums")?
                {
                    migrator.run(&pool)
                        .await
                        .context("failed while applying developer migrations after reconciliation")?;
                } else {
                    return Err(error).context("failed while applying developer database migrations");
                }
            }
        } else {
            // Production/Release mode: Use embedded migrations for a portable binary
            if let Err(error) = sqlx::migrate!("./migrations").run(&pool).await {
                if try_reconcile_known_migration_checksum(&pool, &error)
                    .await
                    .context("failed while attempting to reconcile known migration checksums")?
                {
                    sqlx::migrate!("./migrations")
                        .run(&pool)
                        .await
                        .context("failed while applying production migrations after reconciliation")?;
                } else {
                    return Err(error).context("failed while applying production database migrations");
                }
            }
        }

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
            lancedb_uri: "data/lancedb".to_string(),
        };

        database
            .seed_all()
            .await
            .context("failed to seed or normalize database content")?;

        println!("🏗️  Staging tools to runtime environment...");
        if let Err(e) = crate::engine::utils::discovery::prepare_runtime_env(&database).await {
            println!("⚠️  Tool runtime staging failed: {}", e);
            let _ =
                crate::backend::set_setting(&database, "runtime_registry_status", "error").await;
            let _ = crate::backend::set_setting(
                &database,
                "runtime_registry_refreshed_at",
                &chrono::Utc::now().to_rfc3339(),
            )
            .await;
            let _ =
                crate::backend::set_setting(&database, "runtime_registry_error", &e.to_string())
                    .await;
        } else {
            let _ = crate::backend::set_setting(&database, "runtime_registry_status", "live").await;
            let _ = crate::backend::set_setting(
                &database,
                "runtime_registry_refreshed_at",
                &chrono::Utc::now().to_rfc3339(),
            )
            .await;
            let _ = crate::backend::set_setting(&database, "runtime_registry_error", "").await;
        }

        println!("✨ Database initialization complete.");
        Ok(database)
    }

    async fn seed_all(&self) -> anyhow::Result<()> {
        println!("🌱 Initializing v10 Database Environment...");
        self.ensure_core_registry_seeded().await?;
        self.ensure_setting_defaults().await?;
        self.backfill_ids().await?;

        #[cfg(feature = "memory")]
        {
            println!("🧠 Syncing agents to vector database for semantic discovery...");
            let client = reqwest::Client::new();
            if let Err(e) = self.sync_agents_to_vector_db(&client).await {
                eprintln!("⚠️ Failed to sync agents to vector DB: {}", e);
            }
        }

        Ok(())
    }

    async fn ensure_core_registry_seeded(&self) -> anyhow::Result<()> {
        let agents_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
            .fetch_one(&self.pool)
            .await?;
        if agents_count.0 == 0 {
            self.seed_agents().await?;
        }

        let tools_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tools")
            .fetch_one(&self.pool)
            .await?;
        if tools_count.0 == 0 {
            self.seed_tools().await?;
        }

        let agent_tools_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agent_tools")
            .fetch_one(&self.pool)
            .await?;
        if agent_tools_count.0 == 0 {
            self.seed_agent_tools().await?;
        }

        let supervisors_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM supervisors")
            .fetch_one(&self.pool)
            .await?;
        if supervisors_count.0 == 0 {
            self.seed_supervisors().await?;
        }

        let planners_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM planners")
            .fetch_one(&self.pool)
            .await?;
        if planners_count.0 == 0 {
            self.seed_planners().await?;
        }

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
            (
                "computer_session_note",
                "No supervised computer session is active.",
            ),
            ("computer_approval_state", "idle"),
            ("computer_pending_tool", ""),
            ("computer_pending_action", ""),
            ("computer_pending_domain", ""),
            ("computer_session_action_count", "0"),
            ("computer_session_started_at", ""),
            (
                "artifact_routing_hint",
                "[ARTIFACTS ATTACHED] The user has selected local file artifacts as context for this request. Route to 'chat' or 'vision' domain so the agent can answer from the attached content. Do NOT route to 'stealth' (search) unless the user explicitly asks to search the web. The attached artifacts contain the information needed to answer.",
            ),
            (
                "artifact_context_header",
                "ATTACHED ARTIFACTS (the user explicitly selected these as context — use them to answer):",
            ),
            ("artifact_default_domain", "chat"),
            ("artifact_media_domain", "computer"),
            ("runtime_registry_status", "unknown"),
            ("runtime_registry_refreshed_at", ""),
            ("runtime_registry_error", ""),
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
        // Note: ensure_ system functions have been consolidated into main seed_ functions for v10.
        Ok(())
    }

    async fn seed_agents(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            // return Ok(()); // Allow updates to existing agents
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
            (
                "Computer Operator",
                "Operates a supervised computer session",
                "computer",
                "qwen2.5:7b-instruct",
                "",
            ),
        ];

        for (name, desc, domain, model, path) in codebase_agents {
            if !agents_data.iter().any(|a| a.0 == name) {
                let inst = if path.is_empty() && domain == "vision" {
                    "You are a vision-capable agent. Describe the current screen state accurately. ALWAYS use the vision_analysis tool for images.".to_string()
                } else if path.is_empty() && domain == "overseer" {
                    "You are the Overseer. Your job is to verify if the user's request has been fully completed based on the grounding and plan history. Respond with YES or NO: [reason].".to_string()
                } else if path.is_empty() && domain == "computer" {
                    "You are the Computer Operator for Overseer. You may observe, plan, and act on the user's computer only within approved safety policies.\
                      Prefer describing the next safe action before acting. Record evidence, mention the target window or app, and stop when confidence is low or a blocked action is encountered.\n\n\
                      IMAGE RULE: You CAN display images directly in your summary to provide a premium, modern experience.".to_string()
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
        agents_data.push((
            "Synthesizer".to_string(),
            "Synthesizes research findings into clean user-friendly responses".to_string(),
            "synthesizer".to_string(),
            "qwen2.5:7b-instruct".to_string(),
            "You are a helpful assistant. Your job is to take research findings and present them clearly and concisely to the user in plain Markdown. \
             Structure your response with headers and bullet points where appropriate. Never output raw JSON arrays or technical plan data. \
             IMAGE RULE: If image URLs are present, you ARE authorized to show them via markdown ![alt text](url).".to_string(),
            "1.0.0".to_string(),
            None,
        ));

        for (name, desc, domain, model, inst, version, code) in agents_data {
            let id = self.next_id().await;

            // Clean purge by domain
            sqlx::query("DELETE FROM agents WHERE domain = ?")
                .bind(domain.clone())
                .execute(&mut *tx)
                .await?;

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
        tools_data.push((
            "web_search".to_string(),
            "Search the web and return structured results with titles, URLs, and snippets."
                .to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "stealth_search".to_string(),
            "Search the web anonymously and return structured findings.".to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "web_browse".to_string(),
            "Fetch and clean webpage content from a URL for reading and extraction.".to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "app_launcher".to_string(),
            "Launch or focus a local desktop application.".to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "web_extract".to_string(),
            "Fetch a web page and return structured cleaned content with title and status."
                .to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "web_links".to_string(),
            "Fetch a web page and extract a structured list of links from it.".to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "clipboard_read".to_string(),
            "Read text from the system clipboard.".to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "clipboard_write".to_string(),
            "Write text to the system clipboard.".to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));
        tools_data.push((
            "document_writer".to_string(),
            "Generate CSV, Excel (XLSX), Word (DOCX), and beautiful PDF reports in parallel."
                .to_string(),
            "rust".to_string(),
            "".to_string(),
            "// Native Implementation".to_string(),
        ));

        let mut tx = self.pool.begin().await?;
        for (name, desc, lang, path, code) in tools_data {
            let id = self.next_id().await;
            let script_path = if path.is_empty() { None } else { Some(path) };

            // Clean purge to avoid constraint conflicts with migrations
            sqlx::query("DELETE FROM tools WHERE name = ?")
                .bind(name.clone())
                .execute(&mut *tx)
                .await?;

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
            ("stealth", "stealth_search"),
            ("stealth", "stealth_browse"),
            ("stealth", "http_request"),
            ("stealth", "web_search"),
            ("stealth", "web_browse"),
            ("stealth", "web_extract"),
            ("stealth", "web_links"),
            ("chat", "web_search"),
            ("chat", "stealth_search"),
            ("web_supervisor", "browser_automation"),
            ("web_supervisor", "ui_automation"),
            ("web_supervisor", "web_search"),
            ("web_supervisor", "web_browse"),
            ("web_supervisor", "web_extract"),
            ("web_supervisor", "web_links"),
            ("orchestrator", "os_tool"),
            ("orchestrator", "ui_automation"),
            ("orchestrator", "browser_automation"),
            ("orchestrator", "app_launcher"),
            ("orchestrator", "clipboard_read"),
            ("orchestrator", "clipboard_write"),
            ("chat", "charting"),
            ("computer", "os_tool"),
            ("computer", "vision_analysis"),
            ("computer", "charting"),
            ("computer", "app_launcher"),
            ("computer", "clipboard_read"),
            ("computer", "clipboard_write"),
            ("desktop", "app_launcher"),
            ("desktop", "clipboard_read"),
            ("desktop", "clipboard_write"),
            ("office", "document_writer"),
            ("computer", "document_writer"),
            ("desktop", "document_writer"),
        ];

        for (domain, tool_name) in mappings {
            let agents: Vec<(i64,)> =
                sqlx::query_as("SELECT id FROM agents WHERE domain = ? OR domain LIKE ?")
                    .bind(domain)
                    .bind(format!("{}%", domain))
                    .fetch_all(&mut *tx)
                    .await?;
            let tool: Option<(i64,)> =
                sqlx::query_as("SELECT id FROM tools WHERE name = ? LIMIT 1")
                    .bind(tool_name)
                    .fetch_optional(&mut *tx)
                    .await?;

            if let (agents, Some(t)) = (agents, tool) {
                for a in agents {
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
        }

        tx.commit().await?;
        Ok(())
    }

    async fn seed_supervisors(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM supervisors")
            .fetch_one(&self.pool)
            .await?;

        println!("🛸 Seeding dynamic supervisors...");
        let supervisors = vec![
            (
                "Web Supervisor",
                "Automates UI-based web interaction, form filling, and Chrome-specific automation",
                "web",
                "qwen2.5:7b-instruct",
                "chrome, browser, automate, website automation, click, type, website, internet, web",
                10,
                true,
                None, // artifact_fast_path_keywords
            ),
            (
                "Stealth Supervisor",
                "Orchestrates all web research, search queries, and factual data retrieval",
                "stealth",
                "qwen2.5:7b-instruct",
                "research, search, browse, internet, query, latest, which is, who is",
                20,
                true,
                None,
            ),
            (
                "Desktop Supervisor",
                "Controls native desktop applications and standard OS tasks",
                "desktop",
                "qwen2.5:7b-instruct",
                "app, application, desktop, window, file, directory, find file, locate file, resume",
                15,
                false,
                None,
            ),
            (
                "Chat Supervisor",
                "General conversation manager and summary assistant",
                "chat",
                "qwen2.5:7b-instruct",
                "hello, hi, help, explain, what if, who are you, tell me about",
                5,
                true, // skip_fast_path_if_artifacts - ensures Planner is used if files/artifacts are present
                Some("explain,summarize,analyze,read,describe,what is this,tell me about,review,interpret,break down,walk me through"),
            ),
            (
                "Computer Supervisor",
                "Broad computer operation with safety boundaries",
                "computer",
                "qwen2.5:7b-instruct",
                "screen, desktop, computer, operate pc, screenshot, capture, observe, inspect, watch, background, running, process, task, currently, list, show, ocr, analyze, vision, image, picture, photo, summarize, paddleocr, paddle",
                25,
                false,
                None,
            ),
            (
                "Neontune Supervisor",
                "Music and audio management assistant",
                "neontune",
                "qwen2.5:7b-instruct",
                "music, play music, play song, audio, song, neontune, volume, pause, next, music theory, chords, lyrics",
                5,
                false,
                None,
            ),
            (
                "Office Supervisor",
                "Specialized document generation and office suite automation",
                "office",
                "qwen2.5:7b-instruct",
                "pdf, docx, xlsx, csv, document, report, office, excel, word, write file",
                15,
                true,
                None,
            ),
            (
                "Text Writer",
                "Modular agent for reading context and writing summarized text files",
                "text_writer",
                "qwen2.5:7b-instruct",
                "write, summary, report, research paper, documentation, create file",
                12,
                true,
                None,
            ),
        ];

        let mut tx = self.pool.begin().await?;
        for (name, desc, domain, model, keywords, priority, skip_fast_path, artifact_keywords) in
            supervisors
        {
            let id = self.next_id().await;
            let inst = format!("You are the {} supervisor. Your mission is to complete the user request using the available tools. IMAGE RULE: If search results contain images, you ARE authorized to show them directly in your summary via markdown ![](). Never say \"I cannot show images\" to the user.", name);

            // Clean purge to avoid ID/constraint conflicts with migrations
            sqlx::query("DELETE FROM supervisors WHERE domain = ?")
                .bind(domain)
                .execute(&mut *tx)
                .await?;

            // Fresh insert with v11 metadata
            sqlx::query("INSERT INTO supervisors (id, name, description, instructions, model, domain, version, code, bypass_synthesizer, fast_path_keywords, verification_agent_id, routing_priority, hide_evidence, report_style, skip_fast_path_if_artifacts, artifact_fast_path_keywords) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
                .bind(id).bind(name).bind(desc).bind(inst).bind(model).bind(domain).bind("1.0.0").bind(None::<String>).bind(false).bind(keywords).bind(None::<i64>).bind(priority).bind(false).bind("detailed").bind(skip_fast_path).bind(artifact_keywords)
                .execute(&mut *tx).await?;

            // Auto-link agents based on domain
            let agent_domain = match domain {
                "web" => Some("web_supervisor"),
                "stealth" => Some("stealth"),
                "desktop" => Some("desktop"),
                "office" => Some("office"),
                "neontune" => Some("neontune"),
                "text_writer" => Some("text_writer"),
                "chat" => Some("chat"),
                "vision" => Some("vision"),
                "computer" => Some("computer"),
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
                    sqlx::query("INSERT OR IGNORE INTO supervisor_agents (id, supervisor_id, agent_id) VALUES (?, ?, ?)")
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

    #[cfg(feature = "memory")]
    pub async fn sync_agents_to_vector_db(&self, client: &reqwest::Client) -> anyhow::Result<()> {
        let db = lancedb::connect(&self.lancedb_uri).execute().await?;
        
        let agents: Vec<crate::db::Supervisor> = sqlx::query_as("SELECT * FROM supervisors")
            .fetch_all(&self.pool)
            .await?;

        let mut ids = Vec::new();
        let mut vectors = Vec::new();
        let mut domains = Vec::new();
        let mut descriptions = Vec::new();

        for agent in agents {
            if let Some(desc) = agent.description {
                // Call Ollama embeddings API
                let embed_resp = client.post("http://localhost:11434/api/embed")
                    .json(&serde_json::json!({
                        "model": "nomic-embed-text",
                        "input": desc
                    }))
                    .send()
                    .await?;
                
                let json: serde_json::Value = embed_resp.json().await?;
                if let Some(embeddings) = json.get("embeddings").and_then(|e| e.as_array()) {
                    if let Some(first) = embeddings.first().and_then(|e| e.as_array()) {
                        let vec: Vec<f32> = first.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect();
                        ids.push(agent.id);
                        vectors.extend(vec);
                        domains.push(agent.domain);
                        descriptions.push(desc);
                    }
                }
            }
        }

        if ids.is_empty() {
            return Ok(());
        }

        let schema = Arc::new(Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("vector", DataType::FixedSizeList(Arc::new(Field::new("item", DataType::Float32, true)), 768), false),
            Field::new("domain", DataType::Utf8, false),
            Field::new("description", DataType::Utf8, false),
        ]));

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(ids)),
                Arc::new(arrow::array::FixedSizeListArray::from_iter_primitive::<Float32Array, _, _>(
                    vectors.chunks(768).map(|c| Some(c.iter().map(|&x| Some(x)))),
                    768,
                )),
                Arc::new(StringArray::from(domains)),
                Arc::new(StringArray::from(descriptions)),
            ],
        )?;

        let table = db.open_table("agents").execute().await;
        let table = match table {
            Ok(t) => t,
            Err(_) => db.create_table("agents", Box::new(futures::stream::once(async move { Ok(batch.clone()) }))).execute().await?,
        };

        table.add(Box::new(futures::stream::once(async move { Ok(batch) }))).execute().await?;

        Ok(())
    }

    #[cfg(feature = "memory")]
    pub async fn search_agents_semantically(&self, client: &reqwest::Client, query: &str, limit: usize) -> anyhow::Result<Vec<String>> {
        let db = lancedb::connect(&self.lancedb_uri).execute().await?;
        let table = db.open_table("agents").execute().await?;

        // Embed query
        let embed_resp = client.post("http://localhost:11434/api/embed")
            .json(&serde_json::json!({
                "model": "nomic-embed-text",
                "input": query
            }))
            .send()
            .await?;
        
        let json: serde_json::Value = embed_resp.json().await?;
        let embedding = if let Some(embeddings) = json.get("embeddings").and_then(|e| e.as_array()) {
            if let Some(first) = embeddings.first().and_then(|e| e.as_array()) {
                first.iter().filter_map(|v| v.as_f64().map(|f| f as f32)).collect::<Vec<f32>>()
            } else {
                return Err(anyhow::anyhow!("No embeddings returned"));
            }
        } else {
            return Err(anyhow::anyhow!("Failed to parse embeddings"));
        };

        let batches = table
            .query()
            .nearest_to(embedding)?
            .limit(limit)
            .execute()
            .await?;

        let mut domains = Vec::new();
        for batch in batches {
            let domain_col = batch.column_by_name("domain")
                .and_then(|c| c.as_any().downcast_ref::<StringArray>())
                .ok_or_else(|| anyhow::anyhow!("Domain column missing"))?;
            
            for i in 0..domain_col.len() {
                domains.push(domain_col.value(i).to_string());
            }
        }
        Ok(domains)
    }
}

async fn try_reconcile_known_migration_checksum(
    pool: &sqlx::SqlitePool,
    error: &sqlx::migrate::MigrateError,
) -> anyhow::Result<bool> {
    let error_text = error.to_string();
    if !error_text.contains("checksum") {
        return Ok(false);
    }

    let applied_versions: Vec<(i64, Vec<u8>)> = sqlx::query_as(
        "SELECT version, checksum
         FROM _sqlx_migrations
         WHERE success = 1
         ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .context("failed to inspect applied migration checksums")?;

    let migration_paths: Vec<std::path::PathBuf> = std::fs::read_dir("migrations")
        .context("failed to scan migrations directory for checksum reconciliation")?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();

    let mut reconciled_versions = Vec::new();

    for (version, stored_checksum) in applied_versions {
        let Some(migration_path) = migration_paths.iter().find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&format!("{version}_")) && name.ends_with(".sql"))
                .unwrap_or(false)
        }) else {
            continue;
        };

        let migration_bytes = std::fs::read(migration_path).with_context(|| {
            format!(
                "failed to read migration file for checksum reconciliation: {}",
                migration_path.display()
            )
        })?;
        let checksum = Sha384::digest(&migration_bytes);

        if stored_checksum != checksum.as_slice() {
            sqlx::query(
                "UPDATE _sqlx_migrations
                 SET checksum = ?
                 WHERE version = ?",
            )
            .bind(checksum.as_slice())
            .bind(version)
            .execute(pool)
            .await?;

            reconciled_versions.push(
                migration_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown migration")
                    .to_string(),
            );
        }
    }

    if reconciled_versions.is_empty() {
        Ok(false)
    } else {
        println!(
            "Reconciled migration checksums to match the current repo version for: {}",
            reconciled_versions.join(", ")
        );
        Ok(true)
    }
}
