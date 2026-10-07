// src/db/seed_agents.rs — seed_agents()
use super::Database;

impl Database {
    pub(super) async fn seed_agents(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agents")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            // return Ok(()); // Allow updates to existing agents
        }

        //println!("🌱 Seeding authentic agents from configs and codebase...");

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
}
