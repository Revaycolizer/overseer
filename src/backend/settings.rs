use crate::db::Database;
use std::collections::HashMap;

pub async fn get_setting(db: &Database, key: &str) -> String {
    let res: Result<Option<(String,)>, sqlx::Error> =
        sqlx::query_as("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&db.pool)
            .await;

    match res {
        Ok(Some((v,))) => v,
        _ => "".to_string(),
    }
}

pub async fn get_setting_or(db: &Database, key: &str, default: &str) -> String {
    let value = get_setting(db, key).await;
    if value.is_empty() {
        default.to_string()
    } else {
        value
    }
}

pub async fn get_setting_i64(db: &Database, key: &str, default: i64) -> i64 {
    get_setting(db, key).await.parse::<i64>().unwrap_or(default)
}

pub async fn set_setting(db: &Database, key: &str, value: &str) -> anyhow::Result<()> {
    let id = db.next_id().await;
    sqlx::query("INSERT INTO settings (id, key, value) VALUES (?, ?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = CURRENT_TIMESTAMP")
        .bind(id)
        .bind(key)
        .bind(value)
        .execute(&db.pool)
        .await?;
    Ok(())
}

pub async fn get_all_settings(db: &Database) -> HashMap<String, String> {
    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM settings")
        .fetch_all(&db.pool)
        .await
        .unwrap_or_default();

    rows.into_iter().collect()
}

pub async fn resolve_model_name(
    db: &Database,
    preferred: Option<&str>,
    setting_key: &str,
    hard_default: &str,
) -> String {
    let available_models = super::get_available_models().await;
    let configured = get_setting_or(db, setting_key, hard_default).await;

    let preferred_clean = preferred.map(str::trim).filter(|value| !value.is_empty());
    let configured_clean = configured.trim();

    // V4.17 dynamic model defaults: if the configured value is the
    // sentinel "__use_first_available__" (set by the migration when no
    // explicit model was chosen), prefer the first model in Ollama's
    // /api/tags list. This avoids hardcoding a model name in the seed
    // and matches the user's actual installed models.
    let explicit = configured_clean != "__use_first_available__";
    let configured_useful = explicit.then_some(configured_clean);

    let is_available = |model: &str| {
        available_models.is_empty() || available_models.iter().any(|candidate| candidate == model)
    };

    if let Some(model) = configured_useful {
        if !model.is_empty() && is_available(model) {
            return model.to_string();
        }
    }

    if let Some(model) = preferred_clean {
        if is_available(model) {
            return model.to_string();
        }
    }

    available_models.into_iter().next().unwrap_or_else(|| {
        if configured_clean.is_empty() {
            hard_default.to_string()
        } else {
            configured_clean.to_string()
        }
    })
}

/// V4.17.8: resolve a primary model and walk a fallback chain. Looks
/// up `setting_key` first; if it's empty / missing / not in the
/// available-models list, parses the JSON array stored at
/// `<setting_key>_fallbacks` (e.g. `planner_model_fallbacks =
/// ["qwen3:8b","qwen2.5:7b"]`) and returns the first one that's
/// available. Falls back to `hard_default` if neither is configured.
pub async fn resolve_model_with_fallbacks(
    db: &Database,
    setting_key: &str,
    hard_default: &str,
) -> String {
    let primary = resolve_model_name(db, None, setting_key, hard_default).await;
    let available = super::get_available_models().await;
    let is_available = |m: &str| available.is_empty() || available.iter().any(|c| c == m);

    let raw = get_setting_or(
        db,
        &format!("{}_fallbacks", setting_key),
        "[]",
    )
    .await;
    let parsed: Vec<String> = serde_json::from_str(&raw).unwrap_or_default();
    for candidate in parsed {
        let c = candidate.trim();
        if !c.is_empty() && is_available(c) {
            return c.to_string();
        }
    }

    // Last resort: the hard default. (resolve_model_name already tried
    // `primary` against the available list; if it didn't match
    // anything it falls back to the first available model. If we get
    // here without finding anything better, just return whatever the
    // primary resolved to.)
    primary
}
