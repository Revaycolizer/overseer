// src/db/init.rs — Database::new() constructor
use super::Database;
use anyhow::Context;
use snowflake::SnowflakeIdGenerator;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::ConnectOptions;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::Mutex;

impl Database {
    pub async fn new() -> anyhow::Result<Self> {
        let data_dir = super::get_overseer_data_dir();
        std::fs::create_dir_all(&data_dir).context("failed to create overseer data directory")?;
        let db_path = data_dir.join("overseer.db");

        let mut connection_options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
            .busy_timeout(std::time::Duration::from_secs(30)); // Longer timeout

        connection_options = connection_options
            .disable_statement_logging()
            .log_statements(log::LevelFilter::Off)
            .log_slow_statements(log::LevelFilter::Warn, std::time::Duration::from_secs(30));

       // println!("📜 Running database migrations (Internal Connection)...");
        let pool = SqlitePoolOptions::new()
            .max_connections(1) // Single connection for SQLite is often faster and avoids locks
            .acquire_timeout(std::time::Duration::from_secs(30))
            .acquire_slow_threshold(std::time::Duration::from_secs(30))
            .acquire_slow_level(log::LevelFilter::Off)
            .connect_with(connection_options)
            .await
            .context("failed to connect to SQLite database")?;

        // Synchronous = OFF is safe during migrations for speed, but keep journal_mode = WAL
        sqlx::query("PRAGMA synchronous = OFF")
            .execute(&pool)
            .await
            .context("failed to relax SQLite synchronous mode for migrations")?;

        if cfg!(debug_assertions) {
            println!("🔍 Development mode detected: Using filesystem migrations from ./migrations");
            let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("./migrations"))
                .await
                .context("failed to create filesystem migrator for development")?;
            
            if let Err(error) = migrator.run(&pool).await {
                if super::migration::try_reconcile_known_migration_checksum(&pool, &error)
                    .await
                    .context("failed while attempting to reconcile known migration checksums")?
                {
                    migrator.run(&pool)
                        .await
                        .context("failed while applying developer migrations after reconciliation")?;
                } else {
                    return Err(error).context("failed while applying developer database migrations");
                }
            }
        } else {
            // Production/Release mode: Use embedded migrations for a portable binary
            if let Err(error) = sqlx::migrate!("./migrations").run(&pool).await {
                if super::migration::try_reconcile_known_migration_checksum(&pool, &error)
                    .await
                    .context("failed while attempting to reconcile known migration checksums")?
                {
                    sqlx::migrate!("./migrations")
                        .run(&pool)
                        .await
                        .context("failed while applying production migrations after reconciliation")?;
                } else {
                    return Err(error).context("failed while applying production database migrations");
                }
            }
        }

        // Restore durability
        sqlx::query("PRAGMA synchronous = NORMAL")
            .execute(&pool)
            .await
            .context("failed to restore SQLite synchronous mode after migrations")?;

        // Pool is already created above and migrated

        //println!("📂 Initializing LanceDB...");
        let generator = SnowflakeIdGenerator::new(1, 1);
        let database = Self {
            pool: pool.clone(),
            snowflake: Arc::new(Mutex::new(generator)),
            lancedb_uri: data_dir.join("data").join("lancedb").to_string_lossy().to_string(),
            remote_url: None,
            http_client: reqwest::Client::new(),
        };

        database
            .seed_all()
            .await
            .context("failed to seed or normalize database content")?;

        //println!("🏗️  Staging tools to runtime environment...");
        if let Err(e) = crate::engine::utils::discovery::prepare_runtime_env(&database).await {
            //println!("⚠️  Tool runtime staging failed: {}", e);
            let _ =
                crate::backend::set_setting(&database, "runtime_registry_status", "error").await;
            let _ = crate::backend::set_setting(
                &database,
                "runtime_registry_refreshed_at",
                &chrono::Utc::now().to_rfc3339(),
            )
            .await;
            let _ =
                crate::backend::set_setting(&database, "runtime_registry_error", &e.to_string())
                    .await;
        } else {
            let _ = crate::backend::set_setting(&database, "runtime_registry_status", "live").await;
            let _ = crate::backend::set_setting(
                &database,
                "runtime_registry_refreshed_at",
                &chrono::Utc::now().to_rfc3339(),
            )
            .await;
            let _ = crate::backend::set_setting(&database, "runtime_registry_error", "").await;
        }

        //println!("✨ Database initialization complete.");
        Ok(database)
    }
}
