use sqlx::sqlite::{SqlitePoolOptions, SqliteConnectOptions, SqliteJournalMode}; 
use sqlx::ConnectOptions;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;
use snowflake::SnowflakeIdGenerator;

#[derive(Clone)]
pub struct Database {
    pub pool: sqlx::SqlitePool,
    pub snowflake: Arc<Mutex<SnowflakeIdGenerator>>,
    pub lancedb: lancedb::Connection,
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
            .busy_timeout(std::time::Duration::from_secs(10));

        connection_options = connection_options.disable_statement_logging();

        println!("📜 Opening migration connection...");
        let mut conn = connection_options.connect().await?;

        println!("📜 Running database migrations...");
        sqlx::migrate!("./migrations")
            .run(&mut conn)
            .await?;
        
        drop(conn);

        println!("🔗 Creating database pool...");
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connection_options)
            .await?;

        println!("📂 Initializing LanceDB...");
        let _ = std::fs::create_dir_all("rig_memory");
        let lancedb = lancedb::connect("rig_memory").execute().await?;

        let generator = SnowflakeIdGenerator::new(1, 1);
        let database = Self {
            pool: pool.clone(),
            snowflake: Arc::new(Mutex::new(generator)),
            lancedb,
        };

        database.seed_all().await?;

        println!("✨ Database initialization complete.");
        Ok(database)
    }

    async fn seed_all(&self) -> anyhow::Result<()> {
        self.seed_agents().await?;
        self.seed_tools().await?;
        self.seed_agent_tools().await?;
        Ok(())
    }

    async fn seed_agents(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 { return Ok(()); }

        println!("🌱 Seeding authentic agents from configs and codebase...");
        
        let mut agents_data = Vec::new();

        // 1. Try to load from config/agents/*.json
        if let Ok(entries) = std::fs::read_dir("config/agents") {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&content) {
                            agents_data.push((
                                val["name"].as_str().unwrap_or("Unknown").to_string(),
                                val["description"].as_str().unwrap_or("").to_string(),
                                val["domain"].as_str().unwrap_or("core").to_string(),
                                val["model"].as_str().unwrap_or("qwen2.5:7b-instruct").to_string(),
                                val["instructions"].as_str().unwrap_or("").to_string(),
                                val["version"].as_str().unwrap_or("1.0.0").to_string(),
                            ));
                        }
                    }
                }
            }
        }

        // 2. Add others not in configs with real preambles
        let codebase_agents = vec![
            ("Master Orchestrator", "Primary system planner", "orchestrator", "qwen2.5:7b-instruct", "src/engine/agents/master_planner/preamble.md"),
            ("Web Planner", "Web task planner", "web", "llama3", "src/engine/agents/web/planner_preamble.md"),
            ("Chat Assistant", "General chat and help", "chat", "llama3", "src/engine/agents/chat/preamble.md"),
            ("Vision Assistant", "Image analysis specialist", "vision", "qwen2.5:7b-instruct", ""), // Hardcoded below
        ];

        for (name, desc, domain, model, path) in codebase_agents {
            if !agents_data.iter().any(|a| a.0 == name) {
                let inst = if path.is_empty() && domain == "vision" {
                    "You are a Vision and General Reasoning Assistant. ALWAYS use the vision_analysis tool for images.".to_string()
                } else if !path.is_empty() {
                    std::fs::read_to_string(path).unwrap_or_else(|_| "Default preamble".to_string())
                } else {
                    "Default preamble".to_string()
                };
                agents_data.push((name.to_string(), desc.to_string(), domain.to_string(), model.to_string(), inst, "1.0.0".to_string()));
            }
        }

        let mut tx = self.pool.begin().await?;
        for (name, desc, domain, model, inst, version) in agents_data {
            let id = self.next_id().await;
            sqlx::query("INSERT INTO agents (id, name, description, domain, model, instructions, version) VALUES (?, ?, ?, ?, ?, ?, ?)")
                .bind(id).bind(name).bind(desc).bind(domain).bind(model).bind(inst).bind(version)
                .execute(&mut *tx).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn seed_tools(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tools")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 { return Ok(()); }

        println!("🛠️ Seeding ALL authentic tools from filesystem...");
        let mut tools_data = Vec::new();

        // Recursively scan agents-scripts
        let script_dirs = vec!["web", "stealth", "desktop"];
        for dir in script_dirs {
            let path = format!("agents-scripts/{}", dir);
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if entry.path().extension().and_then(|s| s.to_str()) == Some("py") {
                        if let Some(name) = entry.file_name().to_str() {
                            let tool_name = name.trim_end_matches(".py").to_string();
                            let script_path = format!("agents-scripts/{}/{}", dir, name);
                            let code = std::fs::read_to_string(entry.path()).unwrap_or_default();
                            let description = format!("{} tool for {} domain", tool_name, dir);
                            tools_data.push((tool_name, description, "python".to_string(), script_path, code));
                        }
                    }
                }
            }
        }

        // Add native tools
        tools_data.push(("os_tool".to_string(), "Native system access".to_string(), "rust".to_string(), "".to_string(), "// Native Implementation".to_string()));

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

        if count.0 > 0 { return Ok(()); }

        println!("🔗 Linking agents and tools with comprehensive mappings...");
        let mut tx = self.pool.begin().await?;
        
        let mappings = vec![
            ("web_planner", "browser_automation"),
            ("web_supervisor", "browser_automation"),
            ("stealth", "stealth_search"),
            ("desktop_supervisor", "ui_automation"),
            ("orchestrator", "os_tool"),
            ("orchestrator", "ui_automation"),
            ("orchestrator", "browser_automation"),
        ];

        for (agent_name, tool_name) in mappings {
            let agent: Option<(i64,)> = sqlx::query_as("SELECT id FROM agents WHERE name = ? LIMIT 1").bind(agent_name).fetch_optional(&mut *tx).await?;
            let tool: Option<(i64,)> = sqlx::query_as("SELECT id FROM tools WHERE name = ? LIMIT 1").bind(tool_name).fetch_optional(&mut *tx).await?;

            if let (Some(a), Some(t)) = (agent, tool) {
                sqlx::query("INSERT INTO agent_tools (agent_id, tool_id) VALUES (?, ?)")
                    .bind(a.0).bind(t.0)
                    .execute(&mut *tx).await?;
            }
        }
        
        tx.commit().await?;
        Ok(())
    }

    pub async fn next_id(&self) -> i64 {
        let mut gen = self.snowflake.lock().await;
        gen.real_time_generate()
    }
}
