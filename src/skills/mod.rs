//! Skill system — composable agent+tool+prompt bundles.
//!
//! A skill is a versioned, self-describing package that can include agents,
//! supervisors, tools, and settings overrides. Skills can be installed from
//! local files or a remote registry, and are stored in the database.

pub mod registry;
pub mod loader;

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// A fully resolved, installable skill.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    pub manifest: SkillManifest,
    pub agents: Vec<SkillAgentDef>,
    pub supervisors: Vec<SkillSupervisorDef>,
    pub settings: HashMap<String, String>,
}

/// Metadata about a skill (the manifest file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub dependencies: Vec<SkillDependency>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDependency {
    pub name: String,
    pub version_req: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillAgentDef {
    pub name: String,
    pub description: String,
    pub domain: String,
    pub instructions: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillSupervisorDef {
    pub name: String,
    pub domain: String,
    pub description: String,
    pub instructions: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub routing_priority: i32,
}
