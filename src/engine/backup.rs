//! Backup and disaster recovery utilities.
//!
//! Provides automated SQLite backups, migration dry-run, and point-in-time
//! export/import for conversation history.

use std::path::{Path, PathBuf};

use crate::db::Database;

/// Default number of backups to keep.
const DEFAULT_BACKUP_ROTATION: usize = 7;

/// Create a backup of the SQLite database using `VACUUM INTO`.
///
/// The backup is written to `{data_dir}/backups/overseer-{timestamp}.db`.
/// Old backups beyond `rotation` count are pruned.
pub async fn create_backup(db: &Database, rotation: Option<usize>) -> anyhow::Result<PathBuf> {
    let rotation = rotation.unwrap_or(DEFAULT_BACKUP_ROTATION).max(1);
    let data_dir = crate::db::get_overseer_data_dir();
    let backup_dir = data_dir.join("backups");
    tokio::fs::create_dir_all(&backup_dir).await?;

    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    let backup_path = backup_dir.join(format!("overseer-{}.db", timestamp));

    // VACUUM INTO creates a compact, clean copy
    sqlx::query("VACUUM INTO ?")
    .bind(backup_path.to_string_lossy().as_ref())
    .execute(&db.pool)
    .await?;

    // The database alone cannot decrypt v2 credentials after a disk loss.
    let key = data_dir.join("credentials.key");
    if key.exists() {
        let destination = backup_path.with_extension("key");
        tokio::fs::copy(key, &destination).await?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            tokio::fs::set_permissions(&destination, std::fs::Permissions::from_mode(0o600))
                .await?;
        }
    }

    // Prune old backups
    let mut backups: Vec<PathBuf> = Vec::new();
    let mut entries = tokio::fs::read_dir(&backup_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.extension().map(|e| e == "db").unwrap_or(false) {
            backups.push(path);
        }
    }
    backups.sort();

    while backups.len() > rotation {
        if let Some(oldest) = backups.first() {
            tokio::fs::remove_file(oldest).await?;
            let key = oldest.with_extension("key");
            if key.exists() {
                tokio::fs::remove_file(key).await?;
            }
            backups.remove(0);
        }
    }

    Ok(backup_path)
}

/// Run all pending migrations in a dry-run mode (no writes).
///
/// Returns the list of migrations that WOULD be applied, without applying them.
pub async fn migration_dry_run(db: &Database) -> anyhow::Result<Vec<String>> {
    let migrator = sqlx::migrate::Migrator::new(std::path::Path::new("./migrations")).await?;

    let applied =
        sqlx::query_scalar::<_, String>("SELECT version FROM _sqlx_migrations ORDER BY version")
            .fetch_all(&db.pool)
            .await
            .unwrap_or_default();

    let pending: Vec<String> = migrator
        .iter()
        .filter(|m| !applied.iter().any(|a| a == &m.version.to_string()))
        .map(|m| format!("{} — {}", m.version, m.description))
        .collect();

    Ok(pending)
}

/// Export all conversations to a JSON archive.
pub async fn export_all_conversations(db: &Database) -> anyhow::Result<String> {
    let conversations = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, title, created_at FROM conversations ORDER BY created_at",
    )
    .fetch_all(&db.pool)
    .await?;

    let mut export = Vec::new();
    for (conv_id, title, created_at) in &conversations {
        let messages = sqlx::query_as::<_, (String, String, String)>(
            "SELECT role, content, created_at FROM messages WHERE conversation_id = ? ORDER BY created_at"
        )
        .bind(conv_id)
        .fetch_all(&db.pool)
        .await?;

        export.push(serde_json::json!({
            "id": conv_id,
            "title": title,
            "created_at": created_at,
            "messages": messages.into_iter().map(|(r, c, t)| serde_json::json!({
                "role": r, "content": c, "created_at": t
            })).collect::<Vec<_>>(),
        }));
    }

    Ok(serde_json::to_string_pretty(&serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "exported_at": chrono::Utc::now().to_rfc3339(),
        "conversations": export,
    }))?)
}

/// Import conversations from a JSON archive.
pub async fn import_conversations(db: &Database, json_data: &str) -> anyhow::Result<usize> {
    let data: serde_json::Value = serde_json::from_str(json_data)?;
    let conversations = data["conversations"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing 'conversations' array"))?;

    let mut count = 0;
    for conv in conversations {
        let title = conv["title"].as_str().unwrap_or("Imported");
        let conv_id = db.next_id().await;
        sqlx::query("INSERT INTO conversations (id, title) VALUES (?, ?)")
            .bind(conv_id)
            .bind(title)
            .execute(&db.pool)
            .await?;

        if let Some(messages) = conv["messages"].as_array() {
            for msg in messages {
                let role = msg["role"].as_str().unwrap_or("user");
                let content = msg["content"].as_str().unwrap_or("");
                let msg_id = db.next_id().await;
                sqlx::query(
                    "INSERT INTO messages (id, conversation_id, role, content) VALUES (?, ?, ?, ?)",
                )
                .bind(msg_id)
                .bind(conv_id)
                .bind(role)
                .bind(content)
                .execute(&db.pool)
                .await?;
            }
        }
        count += 1;
    }

    Ok(count)
}
