//! Skill registry — install, list, uninstall, and resolve skills.

use std::collections::HashMap;

use super::{Skill, SkillAgentDef, SkillSupervisorDef};

/// Manages installed skills in the database.
pub struct SkillRegistry {
    db: crate::db::Database,
}

impl SkillRegistry {
    pub fn new(db: crate::db::Database) -> Self {
        Self { db }
    }

    /// List all installed skills.
    pub async fn list_installed(&self) -> Vec<InstalledSkill> {
        sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT name, version, description, installed_at FROM installed_skills ORDER BY name"
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|(name, version, description, installed_at)| InstalledSkill {
            name, version, description, installed_at,
        })
        .collect()
    }

    /// Install a skill, creating all its agents and supervisors.
    pub async fn install(&self, skill: &Skill) -> anyhow::Result<()> {
        // Create agents
        for agent_def in &skill.agents {
            let id = self.db.next_id().await;
            sqlx::query(
                "INSERT INTO agents (id, name, description, domain, instructions, model, tools) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(agent_def.name.clone())
            .bind(agent_def.description.clone())
            .bind(agent_def.domain.clone())
            .bind(agent_def.instructions.clone())
            .bind(agent_def.model.clone().unwrap_or_default())
            .bind(agent_def.tools.join(","))
            .execute(&self.db.pool)
            .await?;
        }

        // Create supervisors
        for sup_def in &skill.supervisors {
            let id = self.db.next_id().await;
            sqlx::query(
                "INSERT INTO supervisors (id, name, domain, description, instructions, model, routing_priority) \
                 VALUES (?, ?, ?, ?, ?, ?, ?)"
            )
            .bind(id)
            .bind(sup_def.name.clone())
            .bind(sup_def.domain.clone())
            .bind(sup_def.description.clone())
            .bind(sup_def.instructions.clone())
            .bind(sup_def.model.clone().unwrap_or_default())
            .bind(sup_def.routing_priority)
            .execute(&self.db.pool)
            .await?;
        }

        // Apply settings
        for (key, value) in &skill.settings {
            let _ = crate::backend::set_setting(&self.db, key, value).await;
        }

        // Record installation
        sqlx::query(
            "INSERT OR REPLACE INTO installed_skills (name, version, description, manifest_json) \
             VALUES (?, ?, ?, ?)"
        )
        .bind(skill.manifest.name.clone())
        .bind(skill.manifest.version.clone())
        .bind(skill.manifest.description.clone())
        .bind(serde_json::to_string(&skill.manifest).unwrap_or_default())
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    /// Uninstall a skill by name.
    pub async fn uninstall(&self, name: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM installed_skills WHERE name = ?")
            .bind(name.to_string())
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    /// Check if a skill is installed.
    pub async fn is_installed(&self, name: &str) -> bool {
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM installed_skills WHERE name = ?"
        )
        .bind(name.to_string())
        .fetch_one(&self.db.pool)
        .await
        .unwrap_or(0) > 0
    }
}

/// Summary of an installed skill for listing.
#[derive(Debug, Clone)]
pub struct InstalledSkill {
    pub name: String,
    pub version: String,
    pub description: String,
    pub installed_at: String,
}
