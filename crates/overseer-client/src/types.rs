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
}

/// A search result highlighting a message within a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationSearchResult {
    pub conversation_id: i64,
    pub conversation_title: String,
    pub message_id: i64,
    pub content: String,
    pub created_at: String,
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
    #[serde(default)]
    pub attachments: Vec<Attachment>,
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
    pub extracted_content: Option<String>,
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
    pub tools: Vec<String>,
    pub instructions: String,
    pub model: String,
    pub domain: String,
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
    pub code: String,
    pub language: String,
    pub script_path: Option<String>,
    pub required_intent: Option<String>,
    pub safety_level: String,
    pub version: String,
}

// ---------------------------------------------------------------------------
// Supervisors
// ---------------------------------------------------------------------------

/// A supervisor that oversees agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Supervisor {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub model: Option<String>,
    pub domain: String,
    pub version: Option<String>,
    pub code: Option<String>,
    pub bypass_synthesizer: bool,
    pub fast_path_keywords: Option<String>,
    pub verification_agent_id: Option<i64>,
    pub routing_priority: i32,
    pub hide_evidence: bool,
    pub report_style: Option<String>,
    pub skip_fast_path_if_artifacts: bool,
    pub artifact_fast_path_keywords: Option<String>,
    pub response_format: String,
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
    pub trigger_type: String,
    pub trigger_config: Option<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// A step within a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub id: i64,
    pub workflow_id: i64,
    pub step_order: i64,
    pub name: String,
    pub prompt: String,
    pub enabled: bool,
    pub created_at: String,
    pub updated_at: String,
}

/// A single execution run of a workflow.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: i64,
    pub workflow_id: i64,
    pub conversation_id: Option<i64>,
    pub status: String,
    pub summary: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
    pub finished_at: Option<String>,
}

/// A single step execution within a workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStepRun {
    pub id: i64,
    pub workflow_run_id: i64,
    pub workflow_step_id: i64,
    pub execution_run_id: Option<i64>,
    pub step_order: i64,
    pub status: String,
    pub output: Option<String>,
    pub error_message: Option<String>,
    pub created_at: String,
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
    pub brief: Option<String>,
    pub status: String,
    pub surface_kind: String,
    pub assigned_domain: Option<String>,
    pub prompt: String,
    pub source_artifact_ids_json: Option<String>,
    pub metadata_json: Option<String>,
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
    pub metadata_json: Option<String>,
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
    pub domain: Option<String>,
    pub step_text: Option<String>,
    pub kind: String,
    pub label: String,
    pub content: Option<String>,
    pub file_path: Option<String>,
    pub content_type: Option<String>,
    pub metadata_json: Option<String>,
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
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub model: Option<String>,
    pub domain: String,
    pub version: Option<String>,
    pub code: Option<String>,
    pub response_format: String,
}

// ---------------------------------------------------------------------------
// Version Control
// ---------------------------------------------------------------------------

/// A single versioned snapshot of an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityVersion {
    pub id: i64,
    pub version_tag: String,
    pub changelog: Option<String>,
    pub created_at: String,
}

// ---------------------------------------------------------------------------
// Health / System
// ---------------------------------------------------------------------------

/// Runtime health summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeHealth {
    pub ollama_reachable: bool,
    pub ollama_endpoint: String,
    pub available_models: Vec<String>,
    pub selected_runtime_model: String,
    pub runtime_model_available: bool,
    pub title_model: String,
    pub title_model_available: bool,
    pub python_command: Option<String>,
    pub protoc_path: Option<String>,
    pub runtime_dir: String,
    pub native_runtime: bool,
}

/// System load snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemLoad {
    pub label: String,
    pub usage: u32,
}

/// Registry status for runtime tool/agent discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeRegistryStatus {
    pub status: String,
    pub refreshed_at: Option<String>,
    pub error: Option<String>,
}

// ---------------------------------------------------------------------------
// Supervision / Computer Control
// ---------------------------------------------------------------------------

/// State of computer supervision.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputerSupervisionState {
    pub supervision_mode: String,
    pub session_state: String,
    pub session_note: String,
    pub allowlist: Vec<String>,
    pub max_actions: i64,
    pub action_count: i64,
    pub timeout_ms: i64,
    pub approval_state: String,
    pub pending_tool: String,
    pub pending_action: String,
    pub pending_domain: String,
    pub session_started_at: Option<String>,
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
    pub id: Option<i64>,
    pub session: Option<BrowserSessionInfo>,
    pub exists: Option<bool>,
}

/// Basic info about a browser session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSessionInfo {
    pub id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: String,
    pub version: String,
    pub uptime_secs: u64,
    pub db_connected: bool,
    pub ollama_reachable: bool,
}
