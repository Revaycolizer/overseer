//! Shared types returned by the Overseer REST API.
//!
//! These mirror the server-side types and are serialised via `serde`.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

/// Profile of an authenticated user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub id: i64,
    pub username: String,
    pub display_name: String,
}

/// Response from login / register.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserProfile,
}

// ---------------------------------------------------------------------------
// Conversations
// ---------------------------------------------------------------------------

/// A conversation thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: i64,
    pub title: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A search result highlighting a message within a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchResult {
    pub conversation_id: i64,
    pub message_id: i64,
    pub snippet: String,
}

// ---------------------------------------------------------------------------
// Messages & Attachments
// ---------------------------------------------------------------------------

/// A single message in a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub hide_evidence: bool,
}

/// A file attached to a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub id: i64,
    pub message_id: i64,
    pub filename: String,
    pub file_path: String,
    pub content_type: String,
    pub size: i64,
}

// ---------------------------------------------------------------------------
// Agents
// ---------------------------------------------------------------------------

/// Information about a registered agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub tools: Vec<ToolInfo>,
    pub instructions: String,
    pub model: String,
    pub version: String,
    pub code: Option<String>,
    pub source_files: Vec<String>,
    pub response_format: String,
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

/// Metadata about a registered tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInfo {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub language: String,
    pub code: String,
    pub version: String,
    pub tool_type: String,
    pub domain: String,
    pub parameters_json: Option<String>,
    pub allowed_agents_json: Option<String>,
    pub safety_level: Option<String>,
}

// ---------------------------------------------------------------------------
// Supervisors
// ---------------------------------------------------------------------------

/// A supervisor that oversees agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supervisor {
    pub id: i64,
    pub name: String,
    pub description: String,
    pub domain: String,
    pub preamble: String,
    pub model: String,
    pub temperature: f64,
    pub is_active: bool,
    pub agents: Vec<AgentInfo>,
}

// ---------------------------------------------------------------------------
// Workflows
// ---------------------------------------------------------------------------

/// A workflow definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A step within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: i64,
    pub workflow_id: i64,
    pub step_order: i32,
    pub name: String,
    pub tool_name: Option<String>,
    pub config_json: Option<String>,
}

/// A single execution run of a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: i64,
    pub workflow_id: i64,
    pub status: String,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub result_summary: Option<String>,
}

/// A single step execution within a workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepRun {
    pub id: i64,
    pub run_id: i64,
    pub step_name: String,
    pub status: String,
    pub output_json: Option<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Work Items
// ---------------------------------------------------------------------------

/// A work item (task) within a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub id: i64,
    pub conversation_id: i64,
    pub title: String,
    pub status: String,
    pub surface_kind: String,
    pub created_at: String,
    pub updated_at: String,
}

/// An event logged against a work item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItemEvent {
    pub id: i64,
    pub work_item_id: i64,
    pub execution_run_id: Option<i64>,
    pub event_type: String,
    pub status: Option<String>,
    pub summary: Option<String>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

/// An artifact produced during a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub id: i64,
    pub conversation_id: i64,
    pub execution_run_id: Option<i64>,
    pub kind: String,
    pub label: String,
    pub content: Option<String>,
    pub file_path: Option<String>,
    pub content_type: Option<String>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Planners
// ---------------------------------------------------------------------------

/// A planner configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Planner {
    pub id: i64,
    pub name: String,
    pub instructions: String,
    pub model: String,
    pub is_active: bool,
}

// ---------------------------------------------------------------------------
// Version Control
// ---------------------------------------------------------------------------

/// A single versioned snapshot of an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityVersion {
    pub id: i64,
    pub entity_type: String,
    pub entity_id: i64,
    pub tag: String,
    pub created_at: String,
    pub description: Option<String>,
}

// ---------------------------------------------------------------------------
// Health / System
// ---------------------------------------------------------------------------

/// Runtime health summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHealth {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub db_connected: bool,
    pub ollama_reachable: bool,
}

/// System load snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLoad {
    pub cpu_percent: f64,
    pub memory_percent: f64,
    pub agent_count: i64,
    pub active_conversations: i64,
}

/// Registry status for runtime tool/agent discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRegistryStatus {
    pub last_refresh: Option<String>,
    pub agent_count: i64,
    pub tool_count: i64,
}

// ---------------------------------------------------------------------------
// Supervision / Computer Control
// ---------------------------------------------------------------------------

/// State of computer supervision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerSupervisionState {
    pub enabled: bool,
    pub allowlist: String,
    pub pending_action: Option<serde_json::Value>,
    pub recent_executions: Vec<String>,
}

// ---------------------------------------------------------------------------
// Credentials
// ---------------------------------------------------------------------------

/// A stored credential (value is never returned).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialSummary {
    pub id: i64,
    pub name: String,
    pub cred_type: String,
}

// ---------------------------------------------------------------------------
// Workflow Templates
// ---------------------------------------------------------------------------

/// A workflow template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowTemplate {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub category: String,
}

// ---------------------------------------------------------------------------
// Browser
// ---------------------------------------------------------------------------

/// Result of a browser session operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionResult {
    pub session: Option<BrowserSessionInfo>,
    pub exists: Option<bool>,
}

/// Basic info about a browser session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionInfo {
    pub id: i64,
}
