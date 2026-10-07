use crate::backend::client::*;
use crate::backend::*;
use crate::db::Database;
use std::collections::HashMap;

pub async fn get_setting(db: &Database, key: &str) -> String {
    if db.is_remote() {
        return db.remote_client().get_setting(key).await;
    }
    crate::backend::settings::get_setting(db, key).await
}

pub async fn get_setting_or(db: &Database, key: &str, default: &str) -> String {
    if db.is_remote() {
        return db.remote_client().get_setting_or(key, default).await;
    }
    crate::backend::settings::get_setting_or(db, key, default).await
}

pub async fn get_setting_i64(db: &Database, key: &str, default: i64) -> i64 {
    if db.is_remote() {
        return db.remote_client().get_setting_i64(key, default).await;
    }
    crate::backend::settings::get_setting_i64(db, key, default).await
}

pub async fn set_setting(db: &Database, key: &str, value: &str) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().set_setting(key, value).await;
    }
    crate::backend::settings::set_setting(db, key, value).await
}

pub async fn get_all_settings(db: &Database) -> HashMap<String, String> {
    if db.is_remote() {
        return db.remote_client().get_all_settings().await;
    }
    crate::backend::settings::get_all_settings(db).await
}

pub async fn resolve_model_name(
    db: &Database,
    preferred: Option<&str>,
    setting_key: &str,
    hard_default: &str,
) -> String {
    if db.is_remote() {
        return db
            .remote_client()
            .resolve_model_name(preferred, setting_key, hard_default)
            .await;
    }
    crate::backend::settings::resolve_model_name(db, preferred, setting_key, hard_default).await
}

/// V4.17.8: model fallback chain. Resolves a primary model from
/// `setting_key`, and if that's missing or unset, walks a JSON
/// array of fallback model names stored in `<setting_key>_fallbacks`
/// (e.g. `planner_model_fallbacks`) and returns the first one that
/// resolves to a non-empty value. Returns `hard_default` if neither
/// the primary nor any fallback is configured.
pub async fn resolve_model_with_fallbacks(
    db: &Database,
    setting_key: &str,
    hard_default: &str,
) -> String {
    if db.is_remote() {
        return db
            .remote_client()
            .resolve_model_with_fallbacks(setting_key, hard_default)
            .await;
    }
    crate::backend::settings::resolve_model_with_fallbacks(db, setting_key, hard_default).await
}

pub async fn get_computer_supervision_state(
    db: &Database,
) -> anyhow::Result<ComputerSupervisionState> {
    if db.is_remote() {
        return db.remote_client().get_computer_supervision_state().await;
    }
    Ok(crate::backend::computer_supervision::get_computer_supervision_state(db).await)
}

pub async fn approve_pending_computer_action(db: &Database) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().approve_pending_computer_action().await;
    }
    crate::backend::computer_supervision::approve_pending_computer_action(db).await
}

pub async fn reject_pending_computer_action(db: &Database) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().reject_pending_computer_action().await;
    }
    crate::backend::computer_supervision::reject_pending_computer_action(db).await
}

pub async fn get_runtime_health(db: &Database) -> RuntimeHealth {
    if db.is_remote() {
        return db.remote_client().get_runtime_health().await;
    }
    crate::backend::health::get_runtime_health(db).await
}

pub async fn get_planners(db: &Database) -> anyhow::Result<Vec<Planner>> {
    if db.is_remote() {
        return db.remote_client().get_planners().await;
    }
    crate::backend::planners::get_planners(db).await
}

pub async fn add_planner(
    db: &Database,
    name: &str,
    instructions: &str,
    model: &str,
) -> anyhow::Result<i64> {
    if db.is_remote() {
        return db
            .remote_client()
            .add_planner(name, instructions, model)
            .await;
    }
    crate::backend::planners::add_planner(db, name, instructions, model).await
}

pub async fn update_planner(db: &Database, planner: Planner) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().update_planner(planner).await;
    }
    crate::backend::planners::update_planner(db, planner).await
}

pub async fn delete_planner(db: &Database, id: i64) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().delete_planner(id).await;
    }
    crate::backend::planners::delete_planner(db, id).await
}
