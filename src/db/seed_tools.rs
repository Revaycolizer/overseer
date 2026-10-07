// src/db/seed_tools.rs — seed_tools() + seed_agent_tools()
use super::Database;

impl Database {
    pub(super) async fn seed_tools(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM tools")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            return Ok(());
        }

        // println!("🛠️ Seeding ALL authentic tools from filesystem...");
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
        let native_tools = [
            ("os_tool", "Native system access"),
            ("vision_analysis", "Analyze images and perform OCR. Supports URLs and local paths."),
            ("charting", "Create beautiful charts (bar, line, pie) from data. Returns SVG."),
            ("web_search", "Search the web and return structured results with titles, URLs, and snippets."),
            ("stealth_search", "Search the web anonymously and return structured findings."),
            ("web_browse", "Fetch and clean webpage content from a URL for reading and extraction."),
            ("app_launcher", "Launch or focus a local desktop application."),
            ("web_extract", "Fetch a web page and return structured cleaned content with title and status."),
            ("web_links", "Fetch a web page and extract a structured list of links from it."),
            ("clipboard_read", "Read text from the system clipboard."),
            ("clipboard_write", "Write text to the system clipboard."),
            ("document_writer", "Generate CSV, Excel (XLSX), Word (DOCX), and beautiful PDF reports in parallel."),
            ("write_file", "Write text content to a file at an absolute path (creates parent dirs). Input: {\"path\": \"...\", \"content\": \"...\"}. Kernel-governed filesystem write; requires approval."),
        ];
        for (name, desc) in native_tools {
            tools_data.push((
                name.to_string(),
                desc.to_string(),
                "rust".to_string(),
                "".to_string(),
                "// Native Implementation".to_string(),
            ));
        }

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

    pub(super) async fn seed_agent_tools(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agent_tools")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            return Ok(());
        }

        //  println!("🔗 Linking agents and tools with comprehensive mappings...");
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
            ("computer", "write_file"),
            ("desktop", "write_file"),
            ("office", "write_file"),
            ("text_writer", "write_file"),
            ("orchestrator", "write_file"),
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
}
