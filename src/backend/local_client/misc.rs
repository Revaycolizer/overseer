use crate::backend::client::*;
use crate::backend::*;
use std::collections::HashMap;
use super::LocalClient;

#[async_trait::async_trait]
impl SettingsClient for LocalClient {
    async fn get_setting(&self, key: &str) -> String { crate::backend::get_setting(&self.db, key).await }
    async fn get_setting_or(&self, key: &str, def: &str) -> String { crate::backend::get_setting_or(&self.db, key, def).await }
    async fn get_setting_i64(&self, key: &str, default: i64) -> i64 { crate::backend::get_setting_i64(&self.db, key, default).await }
    async fn set_setting(&self, key: &str, val: &str) -> anyhow::Result<()> { crate::backend::set_setting(&self.db, key, val).await }
    async fn get_all_settings(&self) -> HashMap<String, String> { crate::backend::get_all_settings(&self.db).await }
    async fn resolve_model_name(&self, prov: Option<&str>, key: &str, def: &str) -> String { crate::backend::resolve_model_name(&self.db, prov, key, def).await }
    async fn resolve_model_with_fallbacks(&self, key: &str, def: &str) -> String { crate::backend::resolve_model_with_fallbacks(&self.db, key, def).await }
}

#[async_trait::async_trait]
impl SupervisionClient for LocalClient {
    async fn get_computer_supervision_state(&self) -> anyhow::Result<ComputerSupervisionState> { crate::backend::get_computer_supervision_state(&self.db).await }
    async fn activate_computer_session(&self, en: bool, al: &str) -> anyhow::Result<()> {
        let state_str = if en { "observing" } else { "idle" };
        crate::backend::activate_computer_session(&self.db, state_str, al).await
    }
    async fn clear_computer_pending_action(&self) -> anyhow::Result<()> { crate::backend::clear_computer_pending_action(&self.db).await }
    async fn stage_computer_approval_request(&self, t: &str, i: &str) -> anyhow::Result<()> {
        let val: serde_json::Value = serde_json::from_str(i).unwrap_or_default();
        let summary = crate::engine::orchestrator::dispatch::tools::summarize_computer_action(t, &val);
        crate::backend::stage_computer_approval_request(&self.db, "computer", t, &summary).await
    }
    async fn approve_pending_computer_action(&self) -> anyhow::Result<()> { crate::backend::approve_pending_computer_action(&self.db).await }
    async fn reject_pending_computer_action(&self) -> anyhow::Result<()> { crate::backend::reject_pending_computer_action(&self.db).await }
    async fn register_computer_action_execution(&self, o: &str) -> anyhow::Result<()> {
        let val: serde_json::Value = serde_json::from_str(o).unwrap_or_default();
        let tool_name = val.get("tool_name").and_then(|v| v.as_str()).unwrap_or("unknown");
        let summary = val.get("summary").and_then(|v| v.as_str()).unwrap_or(o);
        let _ = crate::backend::register_computer_action_execution(&self.db, tool_name, summary).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl RegistryClient for LocalClient {
    async fn refresh_runtime_registry_now(&self) -> anyhow::Result<()> { crate::backend::refresh_runtime_registry_now(&self.db).await }
    async fn get_runtime_registry_status(&self) -> anyhow::Result<RuntimeRegistryStatus> { Ok(crate::backend::get_runtime_registry_status(&self.db).await) }
    async fn get_available_models(&self) -> anyhow::Result<Vec<String>> { Ok(crate::backend::get_available_models().await) }
    async fn get_runtime_health(&self) -> RuntimeHealth { crate::backend::get_runtime_health(&self.db).await }
}

#[async_trait::async_trait]
impl SystemClient for LocalClient {
    async fn get_system_load(&self) -> anyhow::Result<SystemLoad> { Ok(crate::backend::get_system_load().await) }
    async fn open_local_path(&self, p: &str) -> anyhow::Result<()> { crate::backend::open_local_path(p).await }
    async fn export_to_downloads(&self, filename: &str, content: &str) -> anyhow::Result<String> {
        if content.is_empty() {
            crate::backend::export_to_downloads(filename).await
        } else {
            let download_dir = dirs::download_dir()
                .unwrap_or_else(|| std::path::PathBuf::from("assets/exports"))
                .join("Overseer");
            if !download_dir.exists() {
                let _ = std::fs::create_dir_all(&download_dir);
            }
            let path = download_dir.join(filename);
            std::fs::write(&path, content)?;
            Ok(filename.to_string())
        }
    }
    async fn save_chart_export(&self, _cid: i64, ch: &str) -> anyhow::Result<String> {
        let val: serde_json::Value = serde_json::from_str(ch).unwrap_or_default();
        let title = val.get("title").and_then(|v| v.as_str()).unwrap_or("chart").to_string();
        let data = val.get("data").and_then(|v| v.as_str()).unwrap_or(ch).to_string();
        let format = val.get("format").and_then(|v| v.as_str()).unwrap_or("svg").to_string();
        crate::backend::save_chart_export(title, data, format).await.map_err(|e| anyhow::anyhow!(e))
    }
    async fn test_agent_preamble(&self, pr: &str) -> anyhow::Result<String> { crate::backend::test_agent_preamble(pr).await }
}

#[async_trait::async_trait]
impl OrchestrateClient for LocalClient {
    async fn orchestrate(
        &self,
        conversation_id: i64,
        query: &str,
        token_sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    ) -> anyhow::Result<String> {
        let db_inner = self.db.clone();
        let agent_system = self.agent_system.clone();
        let query_str = query.to_string();
        let res = tokio::spawn(async move {
            agent_system
                .orchestrate(&db_inner, conversation_id, &query_str, token_sender)
                .await
        })
        .await?;
        Ok(res)
    }
}
