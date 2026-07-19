//! Remote skill registry — fetch, browse, and install skills from remote sources.
//!
//! Supports two registry backends:
//! - **GitHub-based**: `https://github.com/revaycolizer/overseer-skills`
//! - **Direct URL**: Any HTTP server serving skill archives

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::StandardSkill;

/// Entry in the remote skill index.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndexEntry {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub tags: Vec<String>,
    /// URL to download the skill as a tar.gz archive.
    pub download_url: String,
    /// SHA-256 checksum of the archive.
    pub checksum: String,
}

/// The remote registry index (index.json).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryIndex {
    pub skills: Vec<RegistryIndexEntry>,
}

/// Fetches and resolves skills from a remote registry.
pub struct RemoteRegistry {
    /// Base URL of the registry (e.g. "https://skills.overseer.ai").
    base_url: String,
    /// HTTP client.
    client: reqwest::Client,
}

impl RemoteRegistry {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into(),
            client: reqwest::Client::new(),
        }
    }

    /// Fetch the skill index from the registry.
    pub async fn fetch_index(&self) -> anyhow::Result<RegistryIndex> {
        let url = format!("{}/index.json", self.base_url);
        let resp = self.client.get(&url).send().await?;
        let index: RegistryIndex = resp.json().await?;
        Ok(index)
    }

    /// Search the registry for skills matching a query.
    pub async fn search(&self, query: &str) -> anyhow::Result<Vec<RegistryIndexEntry>> {
        let index = self.fetch_index().await?;
        let lower = query.to_lowercase();
        Ok(index.skills.into_iter().filter(|s| {
            s.name.to_lowercase().contains(&lower)
                || s.description.to_lowercase().contains(&lower)
                || s.tags.iter().any(|t| t.to_lowercase().contains(&lower))
        }).collect())
    }

    /// Download and parse a skill from the registry by name.
    pub async fn download(&self, name: &str) -> anyhow::Result<StandardSkill> {
        let index = self.fetch_index().await?;
        let entry = index.skills.iter()
            .find(|s| s.name == name)
            .ok_or_else(|| anyhow::anyhow!("Skill '{}' not found in registry", name))?;

        // Download the archive
        let archive_bytes = self.client.get(&entry.download_url)
            .send()
            .await?
            .bytes()
            .await?;

        // Verify checksum
        use sha2::{Sha256, Digest};
        let actual_hash = format!("{:x}", Sha256::digest(&archive_bytes));
        if actual_hash != entry.checksum {
            anyhow::bail!("Checksum mismatch for '{}': expected {}, got {}", name, entry.checksum, actual_hash);
        }

        // Extract tar.gz to temp directory and parse SKILL.md
        let temp_dir = tempfile::tempdir()?;
        let tar_gz = flate2::read::GzDecoder::new(&archive_bytes[..]);
        let mut archive = tar::Archive::new(tar_gz);
        archive.unpack(temp_dir.path())?;

        // Find SKILL.md in the extracted files
        let skill_path = find_skill_md(temp_dir.path())
            .ok_or_else(|| anyhow::anyhow!("Downloaded archive contains no SKILL.md"))?;

        let skill = crate::skills::loader::load_skill_from_dir(&skill_path).await
            .map_err(|e| anyhow::anyhow!("Failed to parse downloaded skill: {}", e))?;

        Ok(skill)
    }
}

/// Recursively search for a directory containing SKILL.md.
fn find_skill_md(root: &std::path::Path) -> Option<std::path::PathBuf> {
    if root.join("SKILL.md").exists() {
        return Some(root.to_path_buf());
    }
    if let Ok(entries) = std::fs::read_dir(root) {
        for entry in entries.flatten() {
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Some(found) = find_skill_md(&entry.path()) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Generate a registry index from a local directory of skills.
/// Useful for maintainers building the index before publishing.
pub fn build_index_from_dir(skills_dir: &std::path::Path, base_download_url: &str) -> anyhow::Result<RegistryIndex> {
    use std::fs;

    let mut entries = Vec::new();
    for entry in fs::read_dir(skills_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() { continue; }

        if !path.join("SKILL.md").exists() { continue; }

        // Read SKILL.md to extract metadata
        let content = fs::read_to_string(path.join("SKILL.md"))?;
        if let Ok(skill) = crate::skills::loader::parse_skill_md(&content, &path) {
            entries.push(RegistryIndexEntry {
                name: skill.manifest.name.clone(),
                description: skill.manifest.description.clone(),
                version: skill.manifest.version.clone(),
                author: skill.manifest.author.clone(),
                tags: skill.manifest.tags.clone(),
                download_url: format!("{}/{}/{}.tar.gz", base_download_url, skill.manifest.name, skill.manifest.version),
                checksum: "pending".to_string(), // Set during CI build
            });
        }
    }

    Ok(RegistryIndex { skills: entries })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_index_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let index = build_index_from_dir(dir.path(), "https://example.com/skills").unwrap();
        assert!(index.skills.is_empty());
    }

    #[test]
    fn test_build_index_with_skill() {
        let dir = tempfile::tempdir().unwrap();
        let skill_dir = dir.path().join("test-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(skill_dir.join("SKILL.md"), r#"---
name: test-skill
description: A test skill
author: Tester
tags: [test]
---
# Body
Instructions.
"#).unwrap();

        let index = build_index_from_dir(dir.path(), "https://example.com/skills").unwrap();
        assert_eq!(index.skills.len(), 1);
        assert_eq!(index.skills[0].name, "test-skill");
        assert_eq!(index.skills[0].author, "Tester");
    }
}
