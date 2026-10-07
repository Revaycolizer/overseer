//! Skill registry — install, list, uninstall, and resolve skills.
//!
//! Skills stored in the `installed_skills` table with their full metadata
//! and instructions from SKILL.md.

use super::StandardSkill;

/// Summary of an installed skill for listing.
#[derive(Debug, Clone)]
pub struct InstalledSkill {
    pub name: String,
    pub version: String,
    pub description: String,
    pub tags: Vec<String>,
    pub installed_at: String,
}

impl From<&StandardSkill> for InstalledSkill {
    fn from(s: &StandardSkill) -> Self {
        Self {
            name: s.manifest.name.clone(),
            version: s.manifest.version.clone(),
            description: s.manifest.description.clone(),
            tags: s.manifest.tags.clone(),
            installed_at: chrono::Utc::now().to_rfc3339(),
        }
    }
}

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
        #[derive(sqlx::FromRow)]
        struct Row {
            name: String,
            version: String,
            description: String,
            tags: String,
            installed_at: String,
        }
        let rows = sqlx::query_as::<_, Row>(
            "SELECT name, version, description, tags, installed_at FROM installed_skills ORDER BY name"
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap_or_default();

        rows.into_iter()
            .map(|r| InstalledSkill {
                name: r.name,
                version: r.version,
                description: r.description,
                tags: r
                    .tags
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect(),
                installed_at: r.installed_at,
            })
            .collect()
    }

    /// Install a standard skill (SKILL.md format).
    pub async fn install(&self, skill: &StandardSkill) -> anyhow::Result<()> {
        let instructions_json = serde_json::json!({
            "markdown": skill.instructions,
            "scripts_dir": skill.scripts_dir,
            "references_dir": skill.references_dir,
            "assets_dir": skill.assets_dir,
        });

        sqlx::query(
            "INSERT OR REPLACE INTO installed_skills \
             (name, version, description, tags, metadata_json, instructions_json, author, license, compatibility) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&skill.manifest.name)
        .bind(&skill.manifest.version)
        .bind(&skill.manifest.description)
        .bind(skill.manifest.tags.join(","))
        .bind(serde_json::to_string(&skill.manifest.metadata).unwrap_or_default())
        .bind(instructions_json.to_string())
        .bind(&skill.manifest.author)
        .bind(&skill.manifest.license)
        .bind(&skill.manifest.compatibility)
        .execute(&self.db.pool)
        .await?;

        Ok(())
    }

    /// Uninstall a skill by name.
    pub async fn uninstall(&self, name: &str) -> anyhow::Result<()> {
        sqlx::query("DELETE FROM installed_skills WHERE name = ?")
            .bind(name)
            .execute(&self.db.pool)
            .await?;
        Ok(())
    }

    /// Check if a skill is installed.
    pub async fn is_installed(&self, name: &str) -> bool {
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM installed_skills WHERE name = ?")
            .bind(name)
            .fetch_one(&self.db.pool)
            .await
            .unwrap_or(0);
        count > 0
    }
}
