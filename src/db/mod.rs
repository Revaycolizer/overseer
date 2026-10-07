// src/db/mod.rs — Database struct, types, and re-exports
mod backfill;
mod init;
mod migration;
mod seed_agents;
mod seed_core;
mod seed_supervisors;
mod seed_tools;
#[cfg(feature = "memory")]
mod vector_sync;

pub use migration::try_reconcile_known_migration_checksum;

use serde::{Deserialize, Serialize};
use snowflake::SnowflakeIdGenerator;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Database {
    #[cfg(not(target_arch = "wasm32"))]
    pub pool: sqlx::SqlitePool,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) kernel: Arc<std::sync::OnceLock<overseer_kernel::Kernel>>,
    #[cfg(not(target_arch = "wasm32"))]
    pub snowflake: Arc<Mutex<SnowflakeIdGenerator>>,
    pub lancedb_uri: String,
    pub remote_url: Option<String>,
    pub http_client: reqwest::Client,
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
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut gen = self.snowflake.lock().await;
            return gen.real_time_generate();
        }
        #[cfg(target_arch = "wasm32")]
        0
    }

    pub fn is_remote(&self) -> bool {
        self.remote_url.is_some()
    }

    pub fn remote_client(&self) -> crate::backend::RemoteClient {
        crate::backend::RemoteClient::new(self.remote_url.clone().unwrap_or_default())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn new_remote(url: &str) -> Self {
        let pool = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
        let generator = SnowflakeIdGenerator::new(1, 1);
        Self {
            pool,
            kernel: Arc::new(std::sync::OnceLock::new()),
            snowflake: Arc::new(Mutex::new(generator)),
            lancedb_uri: "".to_string(),
            remote_url: Some(url.to_string()),
            http_client: reqwest::Client::new(),
        }
    }

    #[cfg(target_arch = "wasm32")]
    pub fn new_remote(url: &str) -> Self {
        Self {
            lancedb_uri: "".to_string(),
            remote_url: Some(url.to_string()),
            http_client: reqwest::Client::new(),
        }
    }
}

pub fn get_overseer_data_dir() -> std::path::PathBuf {
    // Allow isolated verification runs and portable installations without
    // opening or migrating the user's normal application database.
    if let Some(path) = std::env::var_os("OVERSEER_DATA_DIR").filter(|path| !path.is_empty()) {
        return std::path::PathBuf::from(path);
    }
    if let Some(mut data_dir) = dirs::data_dir() {
        data_dir.push("overseer");
        data_dir
    } else {
        std::path::PathBuf::from(".")
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn all_migrations_apply_cleanly() {
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect("sqlite::memory:")
            .await
            .expect("failed to create in-memory SQLite pool");

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .expect("migrations should apply without errors");
    }
}
