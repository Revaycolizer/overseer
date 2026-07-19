//! Skill loader — loads skill definitions from filesystem, parses manifests.

use std::path::Path;

use super::{Skill, SkillAgentDef, SkillManifest, SkillSupervisorDef};

/// Load a skill from a directory containing a `skill.json` manifest.
pub async fn load_skill_from_dir(path: &Path) -> anyhow::Result<Skill> {
    let manifest_path = path.join("skill.json");
    let manifest_content = tokio::fs::read_to_string(&manifest_path).await?;
    let manifest: SkillManifest = serde_json::from_str(&manifest_content)?;

    // Load agents
    let agents_dir = path.join("agents");
    let mut agents = Vec::new();
    if agents_dir.exists() {
        let mut entries = tokio::fs::read_dir(&agents_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(agent) = serde_json::from_str::<SkillAgentDef>(&content) {
                        agents.push(agent);
                    }
                }
            }
        }
    }

    // Load supervisors
    let supervisors_dir = path.join("supervisors");
    let mut supervisors = Vec::new();
    if supervisors_dir.exists() {
        let mut entries = tokio::fs::read_dir(&supervisors_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = tokio::fs::read_to_string(&path).await {
                    if let Ok(sup) = serde_json::from_str::<SkillSupervisorDef>(&content) {
                        supervisors.push(sup);
                    }
                }
            }
        }
    }

    Ok(Skill {
        manifest,
        agents,
        supervisors,
        settings: std::collections::HashMap::new(),
    })
}

/// Built-in skills shipped with Overseer.
pub fn builtin_skills() -> Vec<Skill> {
    vec![
        Skill {
            manifest: SkillManifest {
                name: "web-researcher".into(),
                version: "1.0.0".into(),
                description: "Research any topic using web search and synthesis".into(),
                author: "Overseer".into(),
                license: "MIT".into(),
                dependencies: vec![],
                tags: vec!["research".into(), "web".into()],
            },
            agents: vec![
                SkillAgentDef {
                    name: "researcher".into(),
                    description: "Searches the web for information".into(),
                    domain: "web".into(),
                    instructions: "You are a research agent. Search the web thoroughly for the requested information. Always cite your sources.".into(),
                    model: None,
                    tools: vec!["web_search".into(), "web_extract".into(), "web_browse".into()],
                },
            ],
            supervisors: vec![
                SkillSupervisorDef {
                    name: "research-supervisor".into(),
                    domain: "research".into(),
                    description: "Routes research queries to the researcher agent".into(),
                    instructions: "Route research and information-gathering queries to the researcher agent.".into(),
                    model: None,
                    agents: vec!["researcher".into()],
                    routing_priority: 10,
                },
            ],
            settings: std::collections::HashMap::new(),
        },
    ]
}
