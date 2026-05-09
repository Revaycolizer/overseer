// src/db/migration.rs — try_reconcile_known_migration_checksum()
use anyhow::Context;
use sha2::{Digest, Sha384};

pub async fn try_reconcile_known_migration_checksum(
    pool: &sqlx::SqlitePool,
    error: &sqlx::migrate::MigrateError,
) -> anyhow::Result<bool> {
    let error_text = error.to_string();
    if !error_text.contains("checksum") {
        return Ok(false);
    }

    let applied_versions: Vec<(i64, Vec<u8>)> = sqlx::query_as(
        "SELECT version, checksum
         FROM _sqlx_migrations
         WHERE success = 1
         ORDER BY version",
    )
    .fetch_all(pool)
    .await
    .context("failed to inspect applied migration checksums")?;

    let migration_paths: Vec<std::path::PathBuf> = std::fs::read_dir("migrations")
        .context("failed to scan migrations directory for checksum reconciliation")?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect();

    let mut reconciled_versions = Vec::new();

    for (version, stored_checksum) in applied_versions {
        let Some(migration_path) = migration_paths.iter().find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with(&format!("{version}_")) && name.ends_with(".sql"))
                .unwrap_or(false)
        }) else {
            continue;
        };

        let migration_bytes = std::fs::read(migration_path).with_context(|| {
            format!(
                "failed to read migration file for checksum reconciliation: {}",
                migration_path.display()
            )
        })?;
        let checksum = Sha384::digest(&migration_bytes);

        if stored_checksum != checksum.as_slice() {
            sqlx::query(
                "UPDATE _sqlx_migrations
                 SET checksum = ?
                 WHERE version = ?",
            )
            .bind(checksum.as_slice())
            .bind(version)
            .execute(pool)
            .await?;

            reconciled_versions.push(
                migration_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unknown migration")
                    .to_string(),
            );
        }
    }

    if reconciled_versions.is_empty() {
        Ok(false)
    } else {
        println!(
            "Reconciled migration checksums to match the current repo version for: {}",
            reconciled_versions.join(", ")
        );
        Ok(true)
    }
}
