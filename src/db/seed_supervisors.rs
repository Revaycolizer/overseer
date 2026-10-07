// src/db/seed_supervisors.rs — seed_supervisors() + seed_planners()
use super::Database;

impl Database {
    pub(super) async fn seed_supervisors(&self) -> anyhow::Result<()> {
        let _count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM supervisors")
            .fetch_one(&self.pool)
            .await?;

        // println!("🛸 Seeding dynamic supervisors...");
        let supervisors = vec![
            ("Web Supervisor", "Automates UI-based web interaction, form filling, and Chrome-specific automation", "web", "qwen2.5:7b-instruct", "chrome, browser, automate, website automation, click, type, website, internet, web", 10, true, None),
            ("Stealth Supervisor", "Orchestrates all web research, search queries, and factual data retrieval", "stealth", "qwen2.5:7b-instruct", "research, search, browse, internet, query, latest, which is, who is", 20, true, None),
            ("Desktop Supervisor", "Controls native desktop applications and standard OS tasks", "desktop", "qwen2.5:7b-instruct", "app, application, desktop, window, file, directory, find file, locate file, resume", 15, false, None),
            ("Chat Supervisor", "General conversation manager and summary assistant", "chat", "qwen2.5:7b-instruct", "hello, hi, help, explain, what if, who are you, tell me about", 5, true, Some("explain,summarize,analyze,read,describe,what is this,tell me about,review,interpret,break down,walk me through")),
            ("Computer Supervisor", "Broad computer operation with safety boundaries", "computer", "qwen2.5:7b-instruct", "screen, desktop, computer, operate pc, screenshot, capture, observe, inspect, watch, background, running, process, task, currently, list, show, ocr, analyze, vision, image, picture, photo, summarize, paddleocr, paddle", 25, false, None),
            ("Neontune Supervisor", "Music and audio management assistant", "neontune", "qwen2.5:7b-instruct", "music, play music, play song, audio, song, neontune, volume, pause, next, music theory, chords, lyrics", 5, false, None),
            ("Office Supervisor", "Specialized document generation and office suite automation", "office", "qwen2.5:7b-instruct", "pdf, docx, xlsx, csv, document, report, office, excel, word, write file", 15, true, None),
            ("Text Writer", "Modular agent for reading context and writing summarized text files", "text_writer", "qwen2.5:7b-instruct", "write, summary, report, research paper, documentation, create file", 12, true, None),
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

    pub(super) async fn seed_planners(&self) -> anyhow::Result<()> {
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM planners")
            .fetch_one(&self.pool)
            .await?;

        if count.0 > 0 {
            return Ok(());
        }

        //println!("🌱 Seeding Planners...");
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
}
