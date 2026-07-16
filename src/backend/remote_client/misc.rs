use crate::backend::client::*;
use crate::backend::*;
use std::collections::HashMap;
use super::RemoteClient;

#[async_trait::async_trait]
impl SettingsClient for RemoteClient {
    async fn get_setting(&self, k: &str) -> String {
        let url = format!("{}/api/settings?key={}", self.base_url, k);
        match self.http_client.get(&url).send().await {
            Ok(r) => r.json::<Option<String>>().await.unwrap_or_default().unwrap_or_default(),
            Err(_) => "".to_string(),
        }
    }
    async fn get_setting_or(&self, k: &str, d: &str) -> String {
        let val = self.get_setting(k).await;
        if val.is_empty() { d.to_string() } else { val }
    }
    async fn get_setting_i64(&self, k: &str, default: i64) -> i64 {
        self.get_setting(k).await.parse::<i64>().unwrap_or(default)
    }
    async fn set_setting(&self, k: &str, v: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/settings", self.base_url);
        self.http_client.post(&url).json(&serde_json::json!({ "key": k, "value": v })).send().await?;
        Ok(())
    }
    async fn get_all_settings(&self) -> HashMap<String, String> {
        let url = format!("{}/api/settings/all", self.base_url);
        if let Ok(resp) = self.http_client.get(&url).send().await {
            if let Ok(settings) = resp.json::<HashMap<String, String>>().await {
                return settings;
            }
        }
        HashMap::new()
    }
    async fn resolve_model_name(&self, prov: Option<&str>, k: &str, d: &str) -> String {
        let url = format!("{}/api/settings/resolve_model?provider={:?}&key={}&default={}", self.base_url, prov, k, d);
        if let Ok(resp) = self.http_client.get(&url).send().await {
            if let Ok(name) = resp.json::<String>().await {
                return name;
            }
        }
        d.to_string()
    }
    async fn resolve_model_with_fallbacks(&self, k: &str, d: &str) -> String {
        // For now the remote client just delegates to the same
        // endpoint via a richer query string. A real impl would
        // add a /api/settings/resolve_model_with_fallbacks route.
        // For first-cut, fall back to plain resolve_model_name.
        self.resolve_model_name(None, k, d).await
    }
}

#[async_trait::async_trait]
impl SupervisionClient for RemoteClient {
    async fn get_computer_supervision_state(&self) -> anyhow::Result<ComputerSupervisionState> { Ok(self.http_client.get(format!("{}/api/supervision/state", self.base_url)).send().await?.json().await?) }
    async fn activate_computer_session(&self, en: bool, al: &str) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/supervision/activate", self.base_url)).json(&serde_json::json!({ "enabled": en, "allowlist": al })).send().await?; Ok(()) }
    async fn clear_computer_pending_action(&self) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/supervision/clear_pending", self.base_url)).send().await?; Ok(()) }
    async fn stage_computer_approval_request(&self, t: &str, i: &str) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/supervision/stage", self.base_url)).json(&serde_json::json!({ "tool_name": t, "input_json": i })).send().await?; Ok(()) }
    async fn approve_pending_computer_action(&self) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/supervision/approve", self.base_url)).send().await?; Ok(()) }
    async fn reject_pending_computer_action(&self) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/supervision/reject", self.base_url)).send().await?; Ok(()) }
    async fn register_computer_action_execution(&self, o: &str) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/supervision/register_execution", self.base_url)).json(&serde_json::json!({ "output_json": o })).send().await?; Ok(()) }
}

#[async_trait::async_trait]
impl RegistryClient for RemoteClient {
    async fn refresh_runtime_registry_now(&self) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/registry/refresh", self.base_url)).send().await?; Ok(()) }
    async fn get_runtime_registry_status(&self) -> anyhow::Result<RuntimeRegistryStatus> { Ok(self.http_client.get(format!("{}/api/registry/status", self.base_url)).send().await?.json().await?) }
    async fn get_available_models(&self) -> anyhow::Result<Vec<String>> { Ok(self.http_client.get(format!("{}/api/models/available", self.base_url)).send().await?.json().await?) }
    async fn get_runtime_health(&self) -> RuntimeHealth {
        let url = format!("{}/api/health", self.base_url);
        if let Ok(resp) = self.http_client.get(&url).send().await {
            if let Ok(health) = resp.json::<RuntimeHealth>().await {
                return health;
            }
        }
        RuntimeHealth::default()
    }
}

#[async_trait::async_trait]
impl SystemClient for RemoteClient {
    async fn get_system_load(&self) -> anyhow::Result<SystemLoad> { Ok(self.http_client.get(format!("{}/api/system/load", self.base_url)).send().await?.json().await?) }
    async fn open_local_path(&self, p: &str) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/system/open_path", self.base_url)).json(&serde_json::json!({ "path": p })).send().await?; Ok(()) }
    async fn export_to_downloads(&self, n: &str, c: &str) -> anyhow::Result<String> { Ok(self.http_client.post(format!("{}/api/system/export", self.base_url)).json(&serde_json::json!({ "filename": n, "content": c })).send().await?.json().await?) }
    async fn save_chart_export(&self, cid: i64, ch: &str) -> anyhow::Result<String> { Ok(self.http_client.post(format!("{}/api/system/save_chart", self.base_url)).json(&serde_json::json!({ "conversation_id": cid, "chart_json": ch })).send().await?.json().await?) }
    async fn test_agent_preamble(&self, pr: &str) -> anyhow::Result<String> { Ok(self.http_client.post(format!("{}/api/system/test_preamble", self.base_url)).json(&serde_json::json!({ "preamble": pr })).send().await?.json().await?) }
}

#[async_trait::async_trait]
impl OrchestrateClient for RemoteClient {
    async fn orchestrate(
        &self,
        conversation_id: i64,
        query: &str,
        _token_sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    ) -> anyhow::Result<String> {
        // Remote client cannot forward per-token updates over HTTP
        // without a streaming endpoint. The token_sender is dropped
        // here; if per-token UX is needed for remote users, an SSE
        // streaming endpoint would be needed.
        let url = format!("{}/api/orchestrate", self.base_url);
        let payload = serde_json::json!({ "conversation_id": conversation_id, "query": query });
        let res = self.http_client.post(&url).json(&payload).send().await?.text().await?;
        Ok(res)
    }
}
