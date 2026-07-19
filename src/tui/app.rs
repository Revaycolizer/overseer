use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use crate::backend::client::BackendClient;
use crate::backend::{Conversation, Message, SystemLoad};
use crate::backend::computer_supervision::ComputerSupervisionState;
use crate::backend::workflows::models::Workflow;
use crate::db::Supervisor;
use crate::engine::utils::discovery::{AgentInfo, ToolInfo};

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
    pub settings_map: std::collections::HashMap<String, String>,

    // Phase 3: MCP state
    pub mcp_connected: bool,
    pub mcp_server_url: String,
    pub mcp_tools: Vec<crate::mcp::types::McpTool>,
    pub mcp_resources: Vec<crate::mcp::types::McpResource>,
}

impl App {
    pub async fn new(client: Arc<dyn BackendClient + Send + Sync>) -> anyhow::Result<Self> {
        let mut app = Self {
            client,
            conversations: vec![],
            selected_conv_id: None,
            messages: vec![],
            system_load: None,
            active_tab: ActiveTab::Chat,
            input: String::new(),
            input_cursor: 0,
            status_message: "Tab: Ctrl+Left/Right | Chat: Enter send | /help for commands | q: Quit".to_string(),
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
            settings_map: std::collections::HashMap::new(),
            mcp_connected: false,
            mcp_server_url: "http://localhost:8080/mcp".to_string(),
            mcp_tools: vec![],
            mcp_resources: vec![],
        };
        app.refresh_conversations().await?;
        app.refresh_metrics().await?;
        app.activity.bump();
        Ok(app)
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
