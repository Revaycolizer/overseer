use crate::backend::*;
use std::collections::HashMap;

#[async_trait::async_trait]
pub trait SettingsClient {
    async fn get_setting(&self, key: &str) -> String;
    async fn get_setting_or(&self, key: &str, default: &str) -> String;
    async fn get_setting_i64(&self, key: &str, default: i64) -> i64;
    async fn set_setting(&self, key: &str, value: &str) -> anyhow::Result<()>;
    async fn get_all_settings(&self) -> HashMap<String, String>;
    async fn resolve_model_name(&self, provider: Option<&str>, setting_key: &str, default: &str) -> String;
}

#[async_trait::async_trait]
pub trait SupervisionClient {
    async fn get_computer_supervision_state(&self) -> anyhow::Result<ComputerSupervisionState>;
    async fn activate_computer_session(&self, enabled: bool, allowlist_str: &str) -> anyhow::Result<()>;
    async fn clear_computer_pending_action(&self) -> anyhow::Result<()>;
    async fn stage_computer_approval_request(&self, tool_name: &str, input_json: &str) -> anyhow::Result<()>;
    async fn approve_pending_computer_action(&self) -> anyhow::Result<()>;
    async fn reject_pending_computer_action(&self) -> anyhow::Result<()>;
    async fn register_computer_action_execution(&self, output_json: &str) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait RegistryClient {
    async fn refresh_runtime_registry_now(&self) -> anyhow::Result<()>;
    async fn get_runtime_registry_status(&self) -> anyhow::Result<RuntimeRegistryStatus>;
    async fn get_available_models(&self) -> anyhow::Result<Vec<String>>;
    async fn get_runtime_health(&self) -> RuntimeHealth;
}

#[async_trait::async_trait]
pub trait SystemClient {
    async fn get_system_load(&self) -> anyhow::Result<SystemLoad>;
    async fn open_local_path(&self, path: &str) -> anyhow::Result<()>;
    async fn export_to_downloads(&self, filename: &str, content: &str) -> anyhow::Result<String>;
    async fn save_chart_export(&self, conversation_id: i64, chart_json: &str) -> anyhow::Result<String>;
    async fn test_agent_preamble(&self, preamble: &str) -> anyhow::Result<String>;
}

#[async_trait::async_trait]
pub trait WorkItemClient {
    async fn create_work_item(&self, conversation_id: i64, title: &str, brief: Option<&str>, status: &str, surface_kind: &str, assigned_domain: Option<&str>, prompt: &str, source_artifact_ids_json: Option<&str>, metadata_json: Option<&str>) -> anyhow::Result<i64>;
    async fn get_work_items_for_conversation(&self, conversation_id: i64, limit: i64) -> anyhow::Result<Vec<WorkItem>>;
    async fn update_work_item_status(&self, id: i64, status: &str) -> anyhow::Result<()>;
    async fn update_work_item_runtime(&self, id: i64, status: &str, metadata_json: Option<&str>) -> anyhow::Result<()>;
    async fn log_work_item_event(&self, work_item_id: i64, execution_run_id: Option<i64>, event_type: &str, status: Option<&str>, summary: Option<&str>, metadata_json: Option<&str>) -> anyhow::Result<i64>;
    async fn get_work_item_events(&self, work_item_id: i64, limit: i64) -> anyhow::Result<Vec<WorkItemEvent>>;
}

#[async_trait::async_trait]
pub trait ArtifactClient {
    async fn create_artifact(&self, conversation_id: i64, execution_run_id: Option<i64>, domain: Option<&str>, step_text: Option<&str>, kind: &str, label: &str, content: Option<&str>, file_path: Option<&str>, content_type: Option<&str>, metadata_json: Option<&str>) -> anyhow::Result<i64>;
    async fn get_artifacts_for_conversation(&self, conversation_id: i64, limit: i64) -> anyhow::Result<Vec<Artifact>>;
    async fn get_artifacts_by_ids(&self, ids: &[i64]) -> anyhow::Result<Vec<Artifact>>;
    async fn get_artifacts_for_execution_run(&self, execution_run_id: i64) -> anyhow::Result<Vec<Artifact>>;
}

#[async_trait::async_trait]
pub trait PlannerClient {
    async fn get_planners(&self) -> anyhow::Result<Vec<Planner>>;
    async fn add_planner(&self, name: &str, instructions: &str, model: &str) -> anyhow::Result<i64>;
    async fn update_planner(&self, planner: Planner) -> anyhow::Result<()>;
    async fn delete_planner(&self, id: i64) -> anyhow::Result<()>;
}

#[async_trait::async_trait]
pub trait VersionClient {
    async fn publish_version(&self, entity_type: &str, entity_id: i64, tag: &str, description: &str) -> anyhow::Result<()>;
    async fn publish_agent_version(&self, agent_id: i64, tag: &str, description: &str) -> anyhow::Result<()>;
    async fn publish_tool_version(&self, tool_id: i64, tag: &str, description: &str) -> anyhow::Result<()>;
    async fn rollback_version(&self, entity_type: &str, entity_id: i64, tag: &str) -> anyhow::Result<()>;
    async fn get_version_content(&self, entity_type: &str, entity_id: i64, tag: &str) -> anyhow::Result<String>;
    async fn get_version_history(&self, entity_type: &str, entity_id: i64) -> anyhow::Result<Vec<EntityVersion>>;
}

#[async_trait::async_trait]
pub trait OrchestrateClient {
    /// Run the orchestrator on `query` for the given conversation. If
    /// `token_sender` is `Some`, intermediate LLM tokens for the
    /// supervisor reply are pushed through it (V4.17.1 streaming
    /// path). All current callers pass `None`; only chat_input uses
    /// `Some` to feed per-token UI updates.
    async fn orchestrate(
        &self,
        conversation_id: i64,
        query: &str,
        token_sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
    ) -> anyhow::Result<String>;
}
