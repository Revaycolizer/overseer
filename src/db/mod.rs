// src/db/mod.rs — Database struct, types, and re-exports
mod init;
mod seed_core;
mod seed_agents;
mod seed_tools;
mod seed_supervisors;
mod backfill;
mod migration;
#[cfg(feature = "memory")]
mod vector_sync;

pub use migration::try_reconcile_known_migration_checksum;

use serde::{Deserialize, Serialize};
use snowflake::SnowflakeIdGenerator;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Database {
    pub pool: sqlx::SqlitePool,
    pub snowflake: Arc<Mutex<SnowflakeIdGenerator>>,
    pub lancedb_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Supervisor {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub model: Option<String>,
    pub domain: String,
    pub version: Option<String>,
    pub code: Option<String>,
    #[sqlx(default)]
    pub bypass_synthesizer: bool,
    pub fast_path_keywords: Option<String>,
    pub verification_agent_id: Option<i64>,
    pub routing_priority: i32,
    pub hide_evidence: bool,
    pub report_style: Option<String>,
    #[sqlx(default)]
    pub skip_fast_path_if_artifacts: bool,
    pub artifact_fast_path_keywords: Option<String>,
    pub response_format: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct SupervisorAgent {
    pub id: i64,
    pub supervisor_id: i64,
    pub agent_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Planner {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub model: Option<String>,
    pub domain: String,
    pub version: Option<String>,
    pub code: Option<String>,
    pub response_format: String,
}

impl PartialEq for Database {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Database {
    pub async fn next_id(&self) -> i64 {
        let mut gen = self.snowflake.lock().await;
        gen.real_time_generate()
    }
}

pub fn get_overseer_data_dir() -> std::path::PathBuf {
    if let Some(mut data_dir) = dirs::data_dir() {
        data_dir.push("overseer");
        data_dir
    } else {
        std::path::PathBuf::from(".")
    }
}

