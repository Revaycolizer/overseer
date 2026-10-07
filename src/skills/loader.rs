//! Skill loader — loads skill definitions from SKILL.md files.
//!
//! Supports the open-standard directory format:
//!   my-skill/SKILL.md         — YAML frontmatter + markdown instructions
//!   my-skill/scripts/         — executable tools
//!   my-skill/references/      — reference docs
//!   my-skill/assets/          — templates and resources

use std::collections::HashMap;
use std::path::Path;

use super::{SkillManifest, StandardSkill};

/// Error type for skill parsing.
#[derive(Debug)]
pub enum SkillParseError {
    Io(std::io::Error),
    MissingFrontmatter,
    MissingBody,
    InvalidFormat(String),
}

impl std::fmt::Display for SkillParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SkillParseError::Io(e) => write!(f, "IO error: {}", e),
            SkillParseError::MissingFrontmatter => {
                write!(f, "SKILL.md missing YAML frontmatter (---)")
            }
            SkillParseError::MissingBody => {
                write!(f, "SKILL.md missing markdown body after frontmatter")
            }
            SkillParseError::InvalidFormat(e) => write!(f, "Invalid skill format: {}", e),
        }
    }
}

impl std::error::Error for SkillParseError {}

impl From<std::io::Error> for SkillParseError {
    fn from(e: std::io::Error) -> Self {
        SkillParseError::Io(e)
    }
}

/// Minimal YAML frontmatter parser (no dependency on serde_yaml).
///
/// Handles the subset of YAML used in SKILL.md files:
/// - Scalar keys: `key: value`
/// - Indented nested keys under `metadata:`
/// - Multi-line values (no folding)
fn parse_frontmatter(yaml_str: &str) -> Result<SkillManifest, SkillParseError> {
    let mut name = String::new();
    let mut description = String::new();
    let mut compatibility = String::new();
    let mut version = String::new();
    let mut author = String::new();
    let mut license = String::new();
    let mut tags: Vec<String> = Vec::new();
    let mut metadata: HashMap<String, String> = HashMap::new();
    let mut in_metadata = false;
    let mut metadata_key = String::new();

    for line in yaml_str.lines() {
        if line.starts_with('#') || line.trim().is_empty() {
            in_metadata = false;
            continue;
        }

        // Detect top-level keys
        if let Some((key, value)) = line.split_once(':') {
            let trimmed_key = key.trim();
            if !line.starts_with(' ') && !line.starts_with('-') {
                in_metadata = trimmed_key == "metadata";
                let val = value.trim().trim_matches('"').to_string();
                match trimmed_key {
                    "name" => name = val,
                    "description" => description = val,
                    "compatibility" => compatibility = val,
                    "version" => version = val,
                    "author" => author = val,
                    "license" => license = val,
                    "tags" => {
                        tags = value
                            .trim()
                            .trim_matches(&['[', ']'][..])
                            .split(',')
                            .map(|s| s.trim().trim_matches('"').to_string())
                            .filter(|s| !s.is_empty())
                            .collect();
                    }
                    "metadata" => {} // handled by indented parsing below
                    _ if in_metadata => {
                        metadata_key = trimmed_key.to_string();
                        let _ = metadata.insert(trimmed_key.to_string(), val);
                    }
                    _ => {}
                }
                continue;
            }
        }

        // Indented lines under metadata
        if in_metadata && line.starts_with("  ") {
            if let Some((key, value)) = line.trim().split_once(':') {
                let k = key.trim().to_string();
                let v = value.trim().trim_matches('"').to_string();
                metadata.insert(k, v);
            }
        }
    }

    if name.is_empty() {
        return Err(SkillParseError::InvalidFormat(
            "name is required in frontmatter".into(),
        ));
    }
    if description.is_empty() {
        return Err(SkillParseError::InvalidFormat(
            "description is required in frontmatter".into(),
        ));
    }

    Ok(SkillManifest {
        name,
        description,
        compatibility,
        metadata,
        version: if version.is_empty() {
            "1.0.0".into()
        } else {
            version
        },
        author,
        license,
        tags,
    })
}

/// Parse a SKILL.md file into a StandardSkill.
///
/// Format:
/// ```markdown
/// ---
/// name: my-skill
/// description: Does something useful
/// metadata:
///   key: value
/// tags: [tag1, tag2]
/// ---
/// # Instructions
/// Step-by-step guide...
/// ```
pub fn parse_skill_md(content: &str, skill_root: &Path) -> Result<StandardSkill, SkillParseError> {
    let content = content.trim();

    if !content.starts_with("---") {
        return Err(SkillParseError::MissingFrontmatter);
    }

    let rest = &content[3..].trim_start();
    let end = rest
        .find("\n---")
        .ok_or(SkillParseError::MissingFrontmatter)?;

    let yaml_str = &rest[..end];
    let body = rest[end + 4..].trim();

    if body.is_empty() {
        return Err(SkillParseError::MissingBody);
    }

    let manifest = parse_frontmatter(yaml_str)?;

    let scripts_dir = if skill_root.join("scripts").is_dir() {
        Some("scripts".to_string())
    } else {
        None
    };

    let references_dir = if skill_root.join("references").is_dir() {
        Some("references".to_string())
    } else {
        None
    };

    let assets_dir = if skill_root.join("assets").is_dir() {
        Some("assets".to_string())
    } else {
        None
    };

    Ok(StandardSkill {
        manifest,
        instructions: body.to_string(),
        scripts_dir,
        references_dir,
        assets_dir,
    })
}

/// Load a skill from a directory containing SKILL.md.
pub async fn load_skill_from_dir(path: &Path) -> Result<StandardSkill, SkillParseError> {
    let skill_path = path.join("SKILL.md");
    if !skill_path.exists() {
        return Err(SkillParseError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("SKILL.md not found in {:?}", path),
        )));
    }

    let content = tokio::fs::read_to_string(&skill_path).await?;
    parse_skill_md(&content, path)
}

/// Built-in skills shipped with Overseer.
pub fn builtin_skills() -> Vec<StandardSkill> {
    vec![StandardSkill {
        manifest: SkillManifest {
            name: "web-researcher".into(),
            description: "Research any topic using web search and synthesis".into(),
            compatibility: "Overseer 0.1.4+".into(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("agents".into(), "researcher".into());
                m.insert("tools".into(), "web_search,web_extract,web_browse".into());
                m
            },
            version: "1.0.0".into(),
            author: "Overseer".into(),
            license: "MIT".into(),
            tags: vec!["research".into(), "web".into()],
        },
        instructions: "\
# Web Researcher

## Overview
Searches the web for information on any topic and synthesises findings
into a structured report with citations.

## Instructions

1. **Analyze the query** — identify the core research question
2. **Search the web** — use `web_search` to find relevant sources
3. **Extract content** — use `web_extract` to pull detailed information
4. **Cross-reference** — check multiple sources to verify facts
5. **Synthesize** — produce a structured report
"
        .into(),
        scripts_dir: None,
        references_dir: None,
        assets_dir: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_parse_skill_md_valid() {
        let content = r#"---
name: test-skill
description: A test skill
compatibility: cross-platform
metadata:
  agents: tester
  version: "2"
tags: [test, demo]
---
# Test Skill

## Steps
1. Do something
2. Do something else
"#;
        let skill = parse_skill_md(content, Path::new("/tmp")).unwrap();
        assert_eq!(skill.manifest.name, "test-skill");
        assert_eq!(skill.manifest.description, "A test skill");
        assert_eq!(skill.manifest.compatibility, "cross-platform");
        assert_eq!(skill.manifest.metadata.get("agents").unwrap(), "tester");
        assert!(skill.instructions.contains("Do something"));
        assert_eq!(skill.manifest.tags, vec!["test", "demo"]);
    }

    #[test]
    fn test_parse_skill_md_missing_frontmatter() {
        let result = parse_skill_md("# Just markdown\nno frontmatter", Path::new("/tmp"));
        assert!(matches!(
            result.unwrap_err(),
            SkillParseError::MissingFrontmatter
        ));
    }

    #[test]
    fn test_parse_skill_md_missing_body() {
        let result = parse_skill_md("---\nname: empty\n---", Path::new("/tmp"));
        assert!(matches!(result.unwrap_err(), SkillParseError::MissingBody));
    }

    #[test]
    fn test_parse_skill_md_no_metadata() {
        let content = r#"---
name: minimal
description: Minimal skill
---
# Body
Do the thing.
"#;
        let skill = parse_skill_md(content, Path::new("/tmp")).unwrap();
        assert!(skill.manifest.metadata.is_empty());
        assert_eq!(skill.manifest.compatibility, "");
    }

    #[test]
    fn test_parse_frontmatter_tags() {
        let content = r#"---
name: tagged
description: Has tags
tags: [research, web, analysis]
---
# Body
"#;
        let skill = parse_skill_md(content, Path::new("/tmp")).unwrap();
        assert_eq!(skill.manifest.tags, vec!["research", "web", "analysis"]);
    }

    #[test]
    fn test_parse_frontmatter_version_default() {
        let content = r#"---
name: no-version
description: No version specified
---
# Body
"#;
        let skill = parse_skill_md(content, Path::new("/tmp")).unwrap();
        assert_eq!(skill.manifest.version, "1.0.0");
    }
}
