//! Skill system — open-standard agentic skill format.
//!
//! Skills follow the standard directory format:
//!
//! ```text
//! my-skill/
//! ├── SKILL.md         # YAML frontmatter + markdown instructions (required)
//! ├── scripts/         # Executable code the agent can call (optional)
//! ├── references/      # Documentation or reference materials (optional)
//! └── assets/          # Templates and resources (optional)
//! ```
//!
//! The SKILL.md file contains YAML frontmatter (name, description, etc.)
//! followed by detailed markdown instructions that teach the AI how to
//! perform the skill's task.

pub mod loader;
pub mod registry;
pub use registry::InstalledSkill;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A fully resolved, installable skill in the open-standard format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StandardSkill {
    pub manifest: SkillManifest,
    /// The full markdown body from SKILL.md (after frontmatter).
    pub instructions: String,
    /// Paths to optional directories relative to the skill root.
    pub scripts_dir: Option<String>,
    pub references_dir: Option<String>,
    pub assets_dir: Option<String>,
}

/// Metadata parsed from YAML frontmatter in SKILL.md.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    /// Unique skill identifier (kebab-case).
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// What platforms or tools the skill requires.
    #[serde(default)]
    pub compatibility: String,
    /// Arbitrary key-value metadata (e.g. agent types, venue configs).
    #[serde(default)]
    pub metadata: HashMap<String, String>,
    /// Version (semver).
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

fn default_version() -> String {
    "1.0.0".into()
}
