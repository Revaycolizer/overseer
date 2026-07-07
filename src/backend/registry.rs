use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeRegistryStatus {
    pub status: String,
    pub refreshed_at: Option<String>,
    pub error: Option<String>,
}

pub async fn refresh_runtime_registry_now(db: &Database) -> anyhow::Result<()> {
    match crate::engine::utils::discovery::prepare_runtime_env(db).await {
        Ok(_) => {
            let refreshed_at = chrono::Utc::now().to_rfc3339();
            super::set_setting(db, "runtime_registry_status", "live").await?;
            super::set_setting(db, "runtime_registry_refreshed_at", &refreshed_at).await?;
            super::set_setting(db, "runtime_registry_error", "").await?;
            Ok(())
        }
        Err(err) => {
            let refreshed_at = chrono::Utc::now().to_rfc3339();
            let message = err.to_string();
            let _ = super::set_setting(db, "runtime_registry_status", "error").await;
            let _ = super::set_setting(db, "runtime_registry_refreshed_at", &refreshed_at).await;
            let _ = super::set_setting(db, "runtime_registry_error", &message).await;
            Err(err)
        }
    }
}

pub async fn refresh_runtime_registry(db: &Database) {
    if let Err(err) = refresh_runtime_registry_now(db).await {
        println!(
            "Warning: failed to refresh runtime registry after DB change: {}",
            err
        );
    }
}

pub async fn get_runtime_registry_status(db: &Database) -> RuntimeRegistryStatus {
    let status = super::get_setting_or(db, "runtime_registry_status", "unknown").await;
    let refreshed_at = super::get_setting_or(db, "runtime_registry_refreshed_at", "").await;
    let error = super::get_setting_or(db, "runtime_registry_error", "").await;

    RuntimeRegistryStatus {
        status,
        refreshed_at: (!refreshed_at.trim().is_empty()).then_some(refreshed_at),
        error: (!error.trim().is_empty()).then_some(error),
    }
}
