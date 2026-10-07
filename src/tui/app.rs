use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use crate::backend::client::BackendClient;
use crate::backend::computer_supervision::ComputerSupervisionState;
use crate::backend::workflows::models::Workflow;
use crate::backend::{Conversation, Message, SystemLoad};
use crate::db::Database;
use crate::db::Supervisor;
use crate::engine::utils::discovery::{AgentInfo, ToolInfo};
use crate::skills::registry::InstalledSkill;

pub use super::activity::ActivityCounter;
pub use super::tab::ActiveTab;

/// A single action log entry for the Computer Control panel.
#[derive(Clone, Debug)]
pub struct ActionEntry {
    pub status: String,
    pub tool: String,
    pub detail: String,
}

pub struct App {
    pub client: Arc<dyn BackendClient + Send + Sync>,
    pub db: Option<Arc<Database>>,
    pub conversations: Vec<Conversation>,
    pub selected_conv_id: Option<i64>,
    pub messages: Vec<Message>,
    pub system_load: Option<SystemLoad>,
    pub active_tab: ActiveTab,
    pub input: String,
    pub input_cursor: usize,
    pub status_message: String,
    pub is_orchestrating: Arc<AtomicBool>,
    pub orchestrate_error: Arc<std::sync::Mutex<Option<String>>>,
    pub activity: Arc<ActivityCounter>,
    pub last_orchestrate_finished: Arc<std::sync::Mutex<Option<Instant>>>,
    pub current_orchestrate: Arc<std::sync::Mutex<Option<tokio::task::JoinHandle<()>>>>,
    pub search_results: Option<Vec<crate::backend::ConversationSearchResult>>,
    pub search_query: String,

    // Phase 2: Management panel state
    pub agents: Vec<AgentInfo>,
    pub selected_agent_id: Option<i64>,
    pub supervisors: Vec<Supervisor>,
    pub selected_supervisor_id: Option<i64>,
    pub tools: Vec<ToolInfo>,
    pub selected_tool_id: Option<i64>,
    pub workflows: Vec<Workflow>,
    pub selected_workflow_id: Option<i64>,
    pub computer_state: Option<ComputerSupervisionState>,
    pub computer_actions: Vec<ActionEntry>,
    pub available_models: Vec<String>,
    pub selected_model: usize,
    pub model_error: Option<String>,
    pub chat_scroll: u16,
    pub settings_map: std::collections::HashMap<String, String>,

    // Phase 3: MCP state
    pub mcp_connected: bool,
    pub mcp_server_url: String,
    pub mcp_tools: Vec<crate::mcp::types::McpTool>,
    pub mcp_resources: Vec<crate::mcp::types::McpResource>,

    // Phase 4: Skills state
    pub installed_skills: Vec<InstalledSkill>,
    pub skill_registry: Option<crate::skills::registry::SkillRegistry>,

    // Phase 5: UI state
    pub show_help: bool,
    pub show_search: bool,
}

impl App {
    pub async fn new(
        client: Arc<dyn BackendClient + Send + Sync>,
        db: Arc<Database>,
    ) -> anyhow::Result<Self> {
        let mut app = Self {
            client,
            db: Some(db),
            conversations: vec![],
            selected_conv_id: None,
            messages: vec![],
            system_load: None,
            active_tab: ActiveTab::Chat,
            input: String::new(),
            input_cursor: 0,
            status_message: "Enter: send | F6: models | Tab: next view | F1: help | Ctrl+Q: quit"
                .to_string(),
            is_orchestrating: Arc::new(AtomicBool::new(false)),
            orchestrate_error: Arc::new(std::sync::Mutex::new(None)),
            activity: Arc::new(ActivityCounter::default()),
            last_orchestrate_finished: Arc::new(std::sync::Mutex::new(None)),
            current_orchestrate: Arc::new(std::sync::Mutex::new(None)),
            search_results: None,
            search_query: String::new(),
            agents: vec![],
            selected_agent_id: None,
            supervisors: vec![],
            selected_supervisor_id: None,
            tools: vec![],
            selected_tool_id: None,
            workflows: vec![],
            selected_workflow_id: None,
            computer_state: None,
            computer_actions: vec![],
            available_models: vec![],
            selected_model: 0,
            model_error: None,
            chat_scroll: 0,
            settings_map: std::collections::HashMap::new(),
            mcp_connected: false,
            mcp_server_url: "http://localhost:8080/mcp".to_string(),
            mcp_tools: vec![],
            mcp_resources: vec![],
            installed_skills: vec![],
            skill_registry: None,
            show_help: false,
            show_search: false,
        };
        app.refresh_conversations().await?;
        app.refresh_metrics().await?;
        app.refresh_settings_map().await?;
        app.refresh_models().await;
        app.activity.bump();
        Ok(app)
    }

    pub async fn refresh_models(&mut self) {
        match tokio::time::timeout(
            std::time::Duration::from_secs(4),
            self.client.get_available_models(),
        )
        .await
        {
            Ok(Ok(mut models)) => {
                models.sort();
                models.dedup();
                self.selected_model = models
                    .iter()
                    .position(|m| Some(m) == self.settings_map.get("default_chat_model"))
                    .unwrap_or(0);
                self.available_models = models;
                self.model_error = None;
            }
            result => {
                self.available_models.clear();
                self.selected_model = 0;
                self.model_error = Some(match result {
                    Ok(Err(e)) => format!("Cannot load Ollama models: {e}"),
                    _ => "Loading Ollama models timed out. Check Ollama and the daemon.".into(),
                });
            }
        }
    }

    pub async fn select_model(&mut self, model: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.available_models.iter().any(|m| m == model),
            "Model is not installed in Ollama: {model}"
        );
        for (key, value) in [
            ("default_chat_model", model),
            ("planner_model", model),
            ("llm_provider", "ollama"),
        ] {
            self.client.set_setting(key, value).await?;
            self.settings_map.insert(key.into(), value.into());
        }
        self.status_message = format!("Ollama model selected: {model}");
        self.activity.bump();
        Ok(())
    }

    pub fn selected_conv_idx(&self) -> Option<usize> {
        let id = self.selected_conv_id?;
        self.conversations.iter().position(|c| c.id == id)
    }

    pub fn cancel_current_orchestrate(&self) {
        if let Some(handle) = self.current_orchestrate.lock().unwrap().take() {
            handle.abort();
        }
    }
}
