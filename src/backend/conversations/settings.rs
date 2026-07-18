use crate::db::Database;

pub async fn get_conversation_setting(
    db: &Database,
    conversation_id: i64,
    key: &str,
) -> Option<String> {
    sqlx::query_scalar::<_, String>(
        "SELECT value FROM conversation_settings WHERE conversation_id = ? AND key = ?",
    )
    .bind(conversation_id)
    .bind(key)
    .fetch_optional(&db.pool)
    .await
    .ok()
    .flatten()
}

pub async fn set_conversation_setting(
    db: &Database,
    conversation_id: i64,
    key: &str,
    value: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT OR REPLACE INTO conversation_settings (conversation_id, key, value) VALUES (?, ?, ?)",
    )
    .bind(conversation_id)
    .bind(key)
    .bind(value)
    .execute(&db.pool)
    .await?;
    Ok(())
}

pub async fn delete_conversation_setting(
    db: &Database,
    conversation_id: i64,
    key: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "DELETE FROM conversation_settings WHERE conversation_id = ? AND key = ?",
    )
    .bind(conversation_id)
    .bind(key)
    .execute(&db.pool)
    .await?;
    Ok(())
}
