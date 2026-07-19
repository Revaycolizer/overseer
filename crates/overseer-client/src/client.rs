//! Core HTTP client with authentication and request helpers.

use reqwest::{Client as HttpClient, StatusCode};
use std::sync::Arc;
use url::Url;

use crate::error::ClientError;
use crate::types::*;

/// A typed HTTP client for the Overseer REST API.
///
/// # Example
///
/// ```rust,no_run
/// use overseer_client::OverseerClient;
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let client = OverseerClient::new("http://localhost:8080")?
///     .with_token("my-token");
///
/// let conversations = client.list_conversations(10).await?;
/// println!("{conversations:#?}");
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct OverseerClient {
    http: HttpClient,
    base_url: Url,
    token: Option<Arc<str>>,
}

impl OverseerClient {
    /// Create a new client pointing at the given base URL.
    ///
    /// The base URL should be the root of the server, e.g. `http://localhost:8080`.
    pub fn new(base_url: &str) -> Result<Self, ClientError> {
        let base_url = Url::parse(base_url).map_err(|e| ClientError::InvalidUrl(e.to_string()))?;
        let http = HttpClient::builder()
            .user_agent(concat!("overseer-client/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self { http, base_url, token: None })
    }

    /// Set a bearer token to use for authenticated requests.
    pub fn with_token(mut self, token: impl Into<String>) -> Self {
        self.token = Some(Arc::from(token.into()));
        self
    }

    // ── Low-level helpers ──────────────────────────────────────────────

    /// Build a URL relative to the base.
    fn url(&self, path: &str) -> Url {
        // Strip leading slash so join works correctly
        let path = path.trim_start_matches('/');
        self.base_url.join(path).expect("valid URL")
    }

    /// Conditionally attach the Authorization header.
    fn auth_header(&self) -> Option<(&'static str, String)> {
        self.token.as_ref().map(|t| ("Authorization", format!("Bearer {t}")))
    }

    /// Perform a GET request, decode JSON response.
    async fn get_json<T: serde::de::DeserializeOwned>(&self, path: &str) -> Result<T, ClientError> {
        let resp = self.send_get(path).await?;
        self.check_status(resp.status())?;
        resp.json().await.map_err(ClientError::from)
    }



    /// Perform a POST with a JSON body, decode JSON response.
    async fn post_json<B: serde::Serialize + ?Sized, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T, ClientError> {
        let resp = self.send_post(path, body).await?;
        self.check_status(resp.status())?;
        resp.json().await.map_err(ClientError::from)
    }

    /// Perform a POST with a JSON body, returning nothing.
    async fn post_void<B: serde::Serialize + ?Sized>(&self, path: &str, body: &B) -> Result<(), ClientError> {
        let resp = self.send_post(path, body).await?;
        self.check_status(resp.status())?;
        Ok(())
    }

    /// Perform a PATCH with a JSON body, returning nothing.
    async fn patch_void<B: serde::Serialize + ?Sized>(&self, path: &str, body: &B) -> Result<(), ClientError> {
        let mut req = self.http.patch(self.url(path)).json(body);
        if let Some((key, val)) = self.auth_header() {
            req = req.header(key, &val);
        }
        let resp = req.send().await?;
        self.check_status(resp.status())?;
        Ok(())
    }

    /// Perform a DELETE, returning nothing.
    async fn delete_void(&self, path: &str) -> Result<(), ClientError> {
        let mut req = self.http.delete(self.url(path));
        if let Some((key, val)) = self.auth_header() {
            req = req.header(key, &val);
        }
        let resp = req.send().await?;
        let status = resp.status();
        if status.is_success() || status == StatusCode::NO_CONTENT {
            Ok(())
        } else {
            let body = resp.text().await.unwrap_or_default();
            Err(ClientError::Api { status, body })
        }
    }

    /// Check response status and convert non-2xx to an error.
    fn check_status(&self, status: reqwest::StatusCode) -> Result<(), ClientError> {
        if status.is_success() {
            Ok(())
        } else {
            Err(ClientError::Api { status, body: String::new() })
        }
    }

    /// Send GET request with optional auth header.
    async fn send_get(&self, path: &str) -> Result<reqwest::Response, ClientError> {
        let mut req = self.http.get(self.url(path));
        if let Some((key, val)) = self.auth_header() {
            req = req.header(key, &val);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Api { status, body });
        }
        Ok(resp)
    }

    /// Send POST request with JSON body and optional auth header.
    async fn send_post<B: serde::Serialize + ?Sized>(&self, path: &str, body: &B) -> Result<reqwest::Response, ClientError> {
        let mut req = self.http.post(self.url(path)).json(body);
        if let Some((key, val)) = self.auth_header() {
            req = req.header(key, &val);
        }
        let resp = req.send().await?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ClientError::Api { status, body });
        }
        Ok(resp)
    }

    /// Perform a GET and return raw text.
    async fn get_text(&self, path: &str) -> Result<String, ClientError> {
        let resp = self.send_get(path).await?;
        self.check_status(resp.status())?;
        resp.text().await.map_err(ClientError::from)
    }

    // ── Auth ───────────────────────────────────────────────────────────

    /// Register a new user account.
    ///
    /// `POST /v1/auth/register`
    pub async fn register(
        &self,
        username: &str,
        display_name: &str,
        password: &str,
    ) -> Result<AuthResponse, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { username: &'a str, display_name: &'a str, password: &'a str }
        self.post_json("/v1/auth/register", &Body { username, display_name, password }).await
    }

    /// Login with username and password.
    ///
    /// `POST /v1/auth/login`
    pub async fn login(
        &self,
        username: &str,
        password: &str,
    ) -> Result<AuthResponse, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { username: &'a str, password: &'a str }
        self.post_json("/v1/auth/login", &Body { username, password }).await
    }

    /// Logout the current session.
    ///
    /// `POST /v1/auth/logout`
    pub async fn logout(&self) -> Result<(), ClientError> {
        self.post_void("/v1/auth/logout", &serde_json::json!({})).await
    }

    /// Get the current user's profile.
    ///
    /// `GET /v1/auth/me`
    pub async fn me(&self) -> Result<UserProfile, ClientError> {
        self.get_json("/v1/auth/me").await
    }

    // ── Conversations ──────────────────────────────────────────────────

    /// List conversations, limited to `limit` results.
    ///
    /// `GET /api/conversations?limit={limit}`
    pub async fn list_conversations(&self, limit: i64) -> Result<Vec<Conversation>, ClientError> {
        self.get_json(&format!("/api/conversations?limit={limit}")).await
    }

    /// Create a new conversation.
    ///
    /// `POST /api/conversations`
    pub async fn create_conversation(&self, title: &str) -> Result<i64, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { title: &'a str }
        self.post_json("/api/conversations", &Body { title }).await
    }

    /// Update the title of a conversation.
    ///
    /// `POST /api/conversations/update_title`
    pub async fn update_conversation_title(&self, id: i64, title: &str) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { id: i64, title: String }
        self.post_void("/api/conversations/update_title", &Body { id, title: title.into() }).await
    }

    /// Delete a conversation.
    ///
    /// `POST /api/conversations/delete`
    pub async fn delete_conversation(&self, id: i64) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { id: i64 }
        self.post_void("/api/conversations/delete", &Body { id }).await
    }

    /// Search messages across all conversations.
    ///
    /// `GET /api/conversations/search?q={query}`
    pub async fn search_messages(&self, query: &str) -> Result<Vec<ConversationSearchResult>, ClientError> {
        self.get_json(&format!("/api/conversations/search?q={}", urlencoding(query))).await
    }

    // ── Messages ───────────────────────────────────────────────────────

    /// List messages in a conversation.
    ///
    /// `GET /api/messages?conversation_id={id}`
    pub async fn list_messages(&self, conversation_id: i64) -> Result<Vec<Message>, ClientError> {
        self.get_json(&format!("/api/messages?conversation_id={conversation_id}")).await
    }

    /// Send a new message.
    ///
    /// `POST /api/messages`
    pub async fn send_message(
        &self,
        conversation_id: i64,
        role: &str,
        content: &str,
        hide_evidence: bool,
    ) -> Result<i64, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            conversation_id: i64,
            role: &'a str,
            content: &'a str,
            hide_evidence: bool,
        }
        self.post_json("/api/messages", &Body { conversation_id, role, content, hide_evidence }).await
    }

    /// Delete a message.
    ///
    /// `POST /api/messages/delete`
    pub async fn delete_message(&self, id: i64) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { id: i64 }
        self.post_void("/api/messages/delete", &Body { id }).await
    }

    /// List attachments for a message.
    ///
    /// `GET /api/attachments?message_id={id}`
    pub async fn list_attachments(&self, message_id: i64) -> Result<Vec<Attachment>, ClientError> {
        self.get_json(&format!("/api/attachments?message_id={message_id}")).await
    }

    /// Add an attachment to a message.
    ///
    /// `POST /api/attachments`
    pub async fn add_attachment(
        &self,
        message_id: i64,
        filename: &str,
        file_path: &str,
        content_type: &str,
        size: i64,
    ) -> Result<i64, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> {
            message_id: i64,
            filename: &'a str,
            file_path: &'a str,
            content_type: &'a str,
            size: i64,
        }
        self.post_json("/api/attachments", &Body { message_id, filename, file_path, content_type, size }).await
    }

    // ── Agents ─────────────────────────────────────────────────────────

    /// List all agents.
    ///
    /// `GET /api/agents`
    pub async fn list_agents(&self) -> Result<Vec<AgentInfo>, ClientError> {
        self.get_json("/api/agents").await
    }

    /// List all agents (including disabled).
    ///
    /// `GET /api/agents/all`
    pub async fn list_all_agents(&self) -> Result<Vec<AgentInfo>, ClientError> {
        self.get_json("/api/agents/all").await
    }

    /// Create a new agent.
    ///
    /// `POST /api/agents`
    pub async fn create_agent(
        &self,
        name: &str,
        description: &str,
        domain: &str,
    ) -> Result<i64, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { name: &'a str, description: &'a str, domain: &'a str }
        self.post_json("/api/agents", &Body { name, description, domain }).await
    }

    /// Update an existing agent.
    ///
    /// `POST /api/agents/update`
    pub async fn update_agent(&self, agent: &AgentInfo) -> Result<(), ClientError> {
        self.post_void("/api/agents/update", agent).await
    }

    /// Get agent by domain.
    ///
    /// `GET /api/agents/by_domain?domain={domain}`
    pub async fn get_agent_by_domain(&self, domain: &str) -> Result<Option<AgentInfo>, ClientError> {
        self.get_json(&format!("/api/agents/by_domain?domain={}", urlencoding(domain))).await
    }

    // ── Tools ──────────────────────────────────────────────────────────

    /// List all tools.
    ///
    /// `GET /api/tools`
    pub async fn list_tools(&self) -> Result<Vec<ToolInfo>, ClientError> {
        self.get_json("/api/tools").await
    }

    /// Create a new tool.
    ///
    /// `POST /api/tools`
    pub async fn create_tool(
        &self,
        name: &str,
        description: &str,
        language: &str,
    ) -> Result<i64, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { name: &'a str, description: &'a str, language: &'a str }
        self.post_json("/api/tools", &Body { name, description, language }).await
    }

    /// Update an existing tool.
    ///
    /// `POST /api/tools/update`
    pub async fn update_tool(&self, tool: &ToolInfo) -> Result<(), ClientError> {
        self.post_void("/api/tools/update", tool).await
    }

    /// Link a tool to an agent.
    ///
    /// `POST /api/tools/link`
    pub async fn link_tool_to_agent(&self, agent_id: i64, tool_id: i64) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { agent_id: i64, tool_id: i64 }
        self.post_void("/api/tools/link", &Body { agent_id, tool_id }).await
    }

    /// Unlink a tool from an agent.
    ///
    /// `POST /api/tools/unlink`
    pub async fn unlink_tool_from_agent(&self, agent_id: i64, tool_id: i64) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { agent_id: i64, tool_id: i64 }
        self.post_void("/api/tools/unlink", &Body { agent_id, tool_id }).await
    }

    // ── Supervisors ────────────────────────────────────────────────────

    /// List all supervisors.
    ///
    /// `GET /api/supervisors`
    pub async fn list_supervisors(&self) -> Result<Vec<Supervisor>, ClientError> {
        self.get_json("/api/supervisors").await
    }

    /// Create a new supervisor.
    ///
    /// `POST /api/supervisors`
    pub async fn create_supervisor(
        &self,
        name: &str,
        description: &str,
        domain: &str,
    ) -> Result<i64, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { name: &'a str, description: &'a str, domain: &'a str }
        self.post_json("/api/supervisors", &Body { name, description, domain }).await
    }

    /// Update a supervisor.
    ///
    /// `POST /api/supervisors/update`
    pub async fn update_supervisor(&self, supervisor: &Supervisor) -> Result<(), ClientError> {
        self.post_void("/api/supervisors/update", supervisor).await
    }

    /// Delete a supervisor.
    ///
    /// `POST /api/supervisors/delete`
    pub async fn delete_supervisor(&self, id: i64) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { id: i64 }
        self.post_void("/api/supervisors/delete", &Body { id }).await
    }

    /// Get tools belonging to a supervisor.
    ///
    /// `GET /api/supervisors/{id}/tools`
    pub async fn get_supervisor_tools(&self, supervisor_id: i64) -> Result<Vec<ToolInfo>, ClientError> {
        self.get_json(&format!("/api/supervisors/{supervisor_id}/tools")).await
    }

    /// Get agents supervised by a supervisor.
    ///
    /// `GET /api/supervisors/{id}/agents`
    pub async fn get_supervisor_agents(&self, supervisor_id: i64) -> Result<Vec<AgentInfo>, ClientError> {
        self.get_json(&format!("/api/supervisors/{supervisor_id}/agents")).await
    }

    /// Link an agent to a supervisor.
    ///
    /// `POST /api/supervisors/link`
    pub async fn link_agent_to_supervisor(&self, supervisor_id: i64, agent_id: i64) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { supervisor_id: i64, agent_id: i64 }
        self.post_void("/api/supervisors/link", &Body { supervisor_id, agent_id }).await
    }

    /// Unlink an agent from a supervisor.
    ///
    /// `POST /api/supervisors/unlink`
    pub async fn unlink_agent_from_supervisor(&self, supervisor_id: i64, agent_id: i64) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { supervisor_id: i64, agent_id: i64 }
        self.post_void("/api/supervisors/unlink", &Body { supervisor_id, agent_id }).await
    }

    // ── Workflows ──────────────────────────────────────────────────────

    /// List all workflows.
    ///
    /// `GET /api/workflows`
    pub async fn list_workflows(&self) -> Result<Vec<Workflow>, ClientError> {
        self.get_json("/api/workflows").await
    }

    /// Create a new workflow.
    ///
    /// `POST /api/workflows`
    pub async fn create_workflow(&self, name: &str) -> Result<i64, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { name: &'a str }
        self.post_json("/api/workflows", &Body { name }).await
    }

    /// Update a workflow.
    ///
    /// `POST /api/workflows/update`
    pub async fn update_workflow(&self, workflow: &Workflow) -> Result<(), ClientError> {
        self.post_void("/api/workflows/update", workflow).await
    }

    /// Delete a workflow.
    ///
    /// `POST /api/workflows/delete`
    pub async fn delete_workflow(&self, id: i64) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { id: i64 }
        self.post_void("/api/workflows/delete", &Body { id }).await
    }

    /// Get steps for a workflow.
    ///
    /// `GET /api/workflows/{id}/steps`
    pub async fn get_workflow_steps(&self, workflow_id: i64) -> Result<Vec<WorkflowStep>, ClientError> {
        self.get_json(&format!("/api/workflows/{workflow_id}/steps")).await
    }

    /// Add a step to a workflow.
    ///
    /// `POST /api/workflows/{id}/steps`
    pub async fn add_workflow_step(&self, workflow_id: i64) -> Result<i64, ClientError> {
        self.post_json(&format!("/api/workflows/{workflow_id}/steps"), &serde_json::json!({})).await
    }

    /// Update a workflow step.
    ///
    /// `POST /api/workflows/steps/update`
    pub async fn update_workflow_step(&self, step: &WorkflowStep) -> Result<(), ClientError> {
        self.post_void("/api/workflows/steps/update", step).await
    }

    /// Delete a workflow step.
    ///
    /// `POST /api/workflows/steps/delete`
    pub async fn delete_workflow_step(&self, id: i64) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body { id: i64 }
        self.post_void("/api/workflows/steps/delete", &Body { id }).await
    }

    /// Get runs for a workflow.
    ///
    /// `GET /api/workflows/{id}/runs`
    pub async fn get_workflow_runs(&self, workflow_id: i64) -> Result<Vec<WorkflowRun>, ClientError> {
        self.get_json(&format!("/api/workflows/{workflow_id}/runs")).await
    }

    /// Get step runs for a workflow run.
    ///
    /// `GET /api/workflows/runs/{id}/steps`
    pub async fn get_workflow_step_runs(&self, run_id: i64) -> Result<Vec<WorkflowStepRun>, ClientError> {
        self.get_json(&format!("/api/workflows/runs/{run_id}/steps")).await
    }

    /// Run a workflow.
    ///
    /// `POST /api/workflows/{id}/run`
    pub async fn run_workflow(&self, workflow_id: i64) -> Result<i64, ClientError> {
        self.post_json(&format!("/api/workflows/{workflow_id}/run"), &serde_json::json!({})).await
    }

    /// Publish a workflow (creates a version snapshot).
    ///
    /// `POST /v1/workflows/{id}/publish`
    pub async fn publish_workflow(&self, workflow_id: i64) -> Result<(), ClientError> {
        self.post_void(&format!("/v1/workflows/{workflow_id}/publish"), &serde_json::json!({})).await
    }

    /// Get published versions of a workflow.
    ///
    /// `GET /v1/workflows/{id}/versions`
    pub async fn get_workflow_versions(&self, workflow_id: i64) -> Result<Vec<serde_json::Value>, ClientError> {
        self.get_json(&format!("/v1/workflows/{workflow_id}/versions")).await
    }

    /// Restore a workflow to a specific version.
    ///
    /// `POST /v1/workflows/{id}/versions/{v}/restore`
    pub async fn restore_workflow_version(&self, workflow_id: i64, version: i32) -> Result<(), ClientError> {
        self.post_void(
            &format!("/v1/workflows/{workflow_id}/versions/{version}/restore"),
            &serde_json::json!({}),
        ).await
    }

    // ── Work Items ─────────────────────────────────────────────────────

    /// List work items for a conversation.
    ///
    /// `GET /api/work_items?conversation_id={id}`
    pub async fn list_work_items(&self, conversation_id: i64) -> Result<Vec<WorkItem>, ClientError> {
        self.get_json(&format!("/api/work_items?conversation_id={conversation_id}")).await
    }

    /// Create a work item.
    ///
    /// `POST /api/work_items`
    pub async fn create_work_item(&self, params: serde_json::Value) -> Result<i64, ClientError> {
        self.post_json("/api/work_items", &params).await
    }

    /// Update work item status.
    ///
    /// `POST /api/work_items/update_status`
    pub async fn update_work_item_status(&self, id: i64, status: &str) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { id: i64, status: &'a str }
        self.post_void("/api/work_items/update_status", &Body { id, status }).await
    }

    /// Update work item runtime metadata.
    ///
    /// `POST /api/work_items/update_runtime`
    pub async fn update_work_item_runtime(&self, id: i64, status: &str, metadata_json: Option<&str>) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { id: i64, status: &'a str, metadata_json: Option<&'a str> }
        self.post_void("/api/work_items/update_runtime", &Body { id, status, metadata_json }).await
    }

    /// Log an event against a work item.
    ///
    /// `POST /api/work_items/event`
    pub async fn log_work_item_event(&self, params: serde_json::Value) -> Result<i64, ClientError> {
        self.post_json("/api/work_items/event", &params).await
    }

    /// Get events for a work item.
    ///
    /// `GET /api/work_items/{id}/events?limit={limit}`
    pub async fn get_work_item_events(&self, work_item_id: i64, limit: i64) -> Result<Vec<WorkItemEvent>, ClientError> {
        self.get_json(&format!("/api/work_items/{work_item_id}/events?limit={limit}")).await
    }

    // ── Artifacts ──────────────────────────────────────────────────────

    /// List artifacts for a conversation.
    ///
    /// `GET /api/artifacts?conversation_id={id}`
    pub async fn list_artifacts(&self, conversation_id: i64) -> Result<Vec<Artifact>, ClientError> {
        self.get_json(&format!("/api/artifacts?conversation_id={conversation_id}")).await
    }

    /// Create an artifact.
    ///
    /// `POST /api/artifacts`
    pub async fn create_artifact(&self, params: serde_json::Value) -> Result<i64, ClientError> {
        self.post_json("/api/artifacts", &params).await
    }

    /// Get artifacts by their IDs.
    ///
    /// `POST /api/artifacts/by_ids`
    pub async fn get_artifacts_by_ids(&self, ids: &[i64]) -> Result<Vec<Artifact>, ClientError> {
        self.post_json("/api/artifacts/by_ids", ids).await
    }

    /// Get artifacts for an execution run.
    ///
    /// `GET /api/artifacts/by_run?execution_run_id={id}`
    pub async fn get_artifacts_by_run(&self, execution_run_id: i64) -> Result<Vec<Artifact>, ClientError> {
        self.get_json(&format!("/api/artifacts/by_run?execution_run_id={execution_run_id}")).await
    }

    // ── Planners ───────────────────────────────────────────────────────

    /// List all planners.
    ///
    /// `GET /api/planners`
    pub async fn list_planners(&self) -> Result<Vec<Planner>, ClientError> {
        self.get_json("/api/planners").await
    }

    /// Add a planner.
    ///
    /// `POST /api/planners`
    pub async fn add_planner(&self, params: serde_json::Value) -> Result<i64, ClientError> {
        self.post_json("/api/planners", &params).await
    }

    // ── Supervision / Kernel Approvals ─────────────────────────────────

    /// Get current computer supervision state.
    ///
    /// `GET /api/supervision/state`
    pub async fn get_supervision_state(&self) -> Result<ComputerSupervisionState, ClientError> {
        self.get_json("/api/supervision/state").await
    }

    /// Approve pending computer action.
    ///
    /// `POST /api/supervision/approve`
    pub async fn approve_computer_action(&self) -> Result<(), ClientError> {
        self.post_void("/api/supervision/approve", &serde_json::json!({})).await
    }

    /// Reject pending computer action.
    ///
    /// `POST /api/supervision/reject`
    pub async fn reject_computer_action(&self) -> Result<(), ClientError> {
        self.post_void("/api/supervision/reject", &serde_json::json!({})).await
    }

    /// Activate computer supervision session.
    ///
    /// `POST /api/supervision/activate`
    pub async fn activate_supervision(&self, enabled: bool, allowlist: &str) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { enabled: bool, allowlist: &'a str }
        self.post_void("/api/supervision/activate", &Body { enabled, allowlist }).await
    }

    /// Clear pending computer action.
    ///
    /// `POST /api/supervision/clear_pending`
    pub async fn clear_pending_action(&self) -> Result<(), ClientError> {
        self.post_void("/api/supervision/clear_pending", &serde_json::json!({})).await
    }

    /// Stage a computer approval request.
    ///
    /// `POST /api/supervision/stage`
    pub async fn stage_approval_request(&self, params: serde_json::Value) -> Result<(), ClientError> {
        self.post_void("/api/supervision/stage", &params).await
    }

    /// Register a computer action execution.
    ///
    /// `POST /api/supervision/register_execution`
    pub async fn register_computer_execution(&self, params: serde_json::Value) -> Result<(), ClientError> {
        self.post_void("/api/supervision/register_execution", &params).await
    }

    /// List pending kernel approvals.
    ///
    /// `GET /api/kernel/approvals`
    pub async fn list_pending_approvals(&self) -> Result<Vec<serde_json::Value>, ClientError> {
        self.get_json("/api/kernel/approvals").await
    }

    /// Approve a kernel action.
    ///
    /// `POST /api/kernel/approve`
    pub async fn kernel_approve(&self, approval_id: &str) -> Result<serde_json::Value, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { approval_id: &'a str, resolver: &'a str }
        self.post_json("/api/kernel/approve", &Body { approval_id, resolver: "api" }).await
    }

    /// Reject a kernel action.
    ///
    /// `POST /api/kernel/reject`
    pub async fn kernel_reject(&self, approval_id: &str) -> Result<serde_json::Value, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { approval_id: &'a str, resolver: &'a str }
        self.post_json("/api/kernel/reject", &Body { approval_id, resolver: "api" }).await
    }

    /// Emergency stop — cancel waiting approvals and abort in-flight execution.
    ///
    /// `POST /api/kernel/emergency_stop`
    pub async fn kernel_emergency_stop(&self, run_id: Option<i64>) -> Result<serde_json::Value, ClientError> {
        #[derive(serde::Serialize)]
        struct Body { run_id: Option<i64> }
        self.post_json("/api/kernel/emergency_stop", &Body { run_id }).await
    }

    // ── Version Control ────────────────────────────────────────────────

    /// Publish a version of an entity.
    ///
    /// `POST /api/version/publish`
    pub async fn publish_version(&self, params: serde_json::Value) -> Result<(), ClientError> {
        self.post_void("/api/version/publish", &params).await
    }

    /// Publish an agent version.
    ///
    /// `POST /api/version/publish_agent`
    pub async fn publish_agent_version(&self, agent_id: i64, tag: &str, description: &str) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { agent_id: i64, tag: &'a str, description: &'a str }
        self.post_void("/api/version/publish_agent", &Body { agent_id, tag, description }).await
    }

    /// Publish a tool version.
    ///
    /// `POST /api/version/publish_tool`
    pub async fn publish_tool_version(&self, tool_id: i64, tag: &str, description: &str) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { tool_id: i64, tag: &'a str, description: &'a str }
        self.post_void("/api/version/publish_tool", &Body { tool_id, tag, description }).await
    }

    /// Rollback an entity to a specific version.
    ///
    /// `POST /api/version/rollback`
    pub async fn rollback_version(&self, params: serde_json::Value) -> Result<(), ClientError> {
        self.post_void("/api/version/rollback", &params).await
    }

    /// Get version content.
    ///
    /// `GET /api/version/content?entity_type={t}&entity_id={id}&tag={tag}`
    pub async fn get_version_content(&self, entity_type: &str, entity_id: i64, tag: &str) -> Result<String, ClientError> {
        self.get_text(&format!(
            "/api/version/content?entity_type={}&entity_id={}&tag={}",
            urlencoding(entity_type), entity_id, urlencoding(tag),
        )).await
    }

    /// Get version history for an entity.
    ///
    /// `GET /api/version/history?entity_type={t}&entity_id={id}`
    pub async fn get_version_history(&self, entity_type: &str, entity_id: i64) -> Result<Vec<EntityVersion>, ClientError> {
        self.get_json(&format!(
            "/api/version/history?entity_type={}&entity_id={}",
            urlencoding(entity_type), entity_id,
        )).await
    }

    // ── Browser ────────────────────────────────────────────────────────

    /// Create a new browser session.
    ///
    /// `POST /v1/browser/sessions`
    pub async fn create_browser_session(&self) -> Result<serde_json::Value, ClientError> {
        self.post_json("/v1/browser/sessions", &serde_json::json!({})).await
    }

    /// Get browser session info.
    ///
    /// `GET /v1/browser/sessions/{id}`
    pub async fn get_browser_session(&self, session_id: i64) -> Result<serde_json::Value, ClientError> {
        self.get_json(&format!("/v1/browser/sessions/{session_id}")).await
    }

    /// Close a browser session.
    ///
    /// `DELETE /v1/browser/sessions/{id}`
    pub async fn close_browser_session(&self, session_id: i64) -> Result<(), ClientError> {
        self.delete_void(&format!("/v1/browser/sessions/{session_id}")).await
    }

    /// Navigate a browser session to a URL.
    ///
    /// `POST /v1/browser/sessions/{id}/navigate`
    pub async fn browser_navigate(&self, session_id: i64, url: &str) -> Result<serde_json::Value, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { url: &'a str }
        self.post_json(&format!("/v1/browser/sessions/{session_id}/navigate"), &Body { url }).await
    }

    /// Take a screenshot of a browser session.
    ///
    /// `GET /v1/browser/sessions/{id}/screenshot`
    pub async fn browser_screenshot(&self, session_id: i64) -> Result<serde_json::Value, ClientError> {
        self.get_json(&format!("/v1/browser/sessions/{session_id}/screenshot")).await
    }

    /// Set action policy for a browser session.
    ///
    /// `POST /v1/browser/sessions/{id}/policy`
    pub async fn browser_set_policy(&self, session_id: i64, policy: serde_json::Value) -> Result<(), ClientError> {
        self.post_void(&format!("/v1/browser/sessions/{session_id}/policy"), &policy).await
    }

    /// Get audit log for a browser session.
    ///
    /// `GET /v1/browser/sessions/{id}/audit`
    pub async fn browser_audit_log(&self, session_id: i64) -> Result<Vec<serde_json::Value>, ClientError> {
        self.get_json(&format!("/v1/browser/sessions/{session_id}/audit")).await
    }

    /// Get browser memory report.
    ///
    /// `GET /v1/browser/memory`
    pub async fn browser_memory_report(&self) -> Result<serde_json::Value, ClientError> {
        self.get_json("/v1/browser/memory").await
    }

    // ── Browser Recording ──────────────────────────────────────────────

    /// Start recording a browser session.
    ///
    /// `POST /v1/browser/sessions/{id}/recording/start`
    pub async fn start_recording(&self, session_id: i64, quality: u8) -> Result<serde_json::Value, ClientError> {
        #[derive(serde::Serialize)]
        struct Body { quality: u8 }
        self.post_json(&format!("/v1/browser/sessions/{session_id}/recording/start"), &Body { quality }).await
    }

    /// Stop recording a browser session.
    ///
    /// `POST /v1/browser/sessions/{id}/recording/stop`
    pub async fn stop_recording(&self, session_id: i64) -> Result<serde_json::Value, ClientError> {
        self.post_json(&format!("/v1/browser/sessions/{session_id}/recording/stop"), &serde_json::json!({})).await
    }

    /// Get recording data for a browser session.
    ///
    /// `GET /v1/browser/sessions/{id}/recording`
    pub async fn get_recording(&self, session_id: i64) -> Result<serde_json::Value, ClientError> {
        self.get_json(&format!("/v1/browser/sessions/{session_id}/recording")).await
    }

    /// Get recording events for a browser session.
    ///
    /// `GET /v1/browser/sessions/{id}/recording/events`
    pub async fn get_recording_events(&self, session_id: i64) -> Result<serde_json::Value, ClientError> {
        self.get_json(&format!("/v1/browser/sessions/{session_id}/recording/events")).await
    }

    /// Resume a browser HITL session.
    ///
    /// `POST /v1/browser/sessions/{id}/resume`
    pub async fn resume_browser_session(&self, session_id: i64) -> Result<(), ClientError> {
        self.post_void(&format!("/v1/browser/sessions/{session_id}/resume"), &serde_json::json!({})).await
    }

    // ── Credentials ────────────────────────────────────────────────────

    /// List all stored credentials.
    ///
    /// `GET /v1/credentials`
    pub async fn list_credentials(&self) -> Result<Vec<CredentialSummary>, ClientError> {
        self.get_json("/v1/credentials").await
    }

    /// Create a new credential.
    ///
    /// `POST /v1/credentials`
    pub async fn create_credential(&self, name: &str, value: &str, cred_type: Option<&str>) -> Result<CredentialSummary, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { name: &'a str, value: &'a str, cred_type: Option<&'a str> }
        self.post_json("/v1/credentials", &Body { name, value, cred_type }).await
    }

    /// Update a credential.
    ///
    /// `PATCH /v1/credentials/{id}`
    pub async fn update_credential(&self, id: i64, name: Option<&str>, value: Option<&str>) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { name: Option<&'a str>, value: Option<&'a str> }
        self.patch_void(&format!("/v1/credentials/{id}"), &Body { name, value }).await
    }

    /// Delete a credential.
    ///
    /// `DELETE /v1/credentials/{id}`
    pub async fn delete_credential(&self, id: i64) -> Result<(), ClientError> {
        self.delete_void(&format!("/v1/credentials/{id}")).await
    }

    // ── Workflow Templates ─────────────────────────────────────────────

    /// List all workflow templates.
    ///
    /// `GET /v1/workflow-templates`
    pub async fn list_workflow_templates(&self) -> Result<Vec<WorkflowTemplate>, ClientError> {
        self.get_json("/v1/workflow-templates").await
    }

    /// Create a workflow template.
    ///
    /// `POST /v1/workflow-templates`
    pub async fn create_workflow_template(&self, params: serde_json::Value) -> Result<WorkflowTemplate, ClientError> {
        self.post_json("/v1/workflow-templates", &params).await
    }

    /// Install a workflow template as a workflow.
    ///
    /// `POST /v1/workflow-templates/{id}/install`
    pub async fn install_workflow_template(&self, template_id: i64) -> Result<i64, ClientError> {
        self.post_json(&format!("/v1/workflow-templates/{template_id}/install"), &serde_json::json!({})).await
    }

    // ── Settings ───────────────────────────────────────────────────────

    /// Get a single setting value.
    ///
    /// `GET /api/settings?key={key}`
    pub async fn get_setting(&self, key: &str) -> Result<Option<String>, ClientError> {
        self.get_json(&format!("/api/settings?key={}", urlencoding(key))).await
    }

    /// Set a single setting value.
    ///
    /// `POST /api/settings`
    pub async fn set_setting(&self, key: &str, value: &str) -> Result<(), ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { key: &'a str, value: &'a str }
        self.post_void("/api/settings", &Body { key, value }).await
    }

    /// Get all settings.
    ///
    /// `GET /api/settings/all`
    pub async fn get_all_settings(&self) -> Result<std::collections::HashMap<String, String>, ClientError> {
        self.get_json("/api/settings/all").await
    }

    // ── System / Health ────────────────────────────────────────────────

    /// Get server health.
    ///
    /// `GET /health`
    pub async fn health(&self) -> Result<RuntimeHealth, ClientError> {
        self.get_json("/health").await
    }

    /// Get system load.
    ///
    /// `GET /api/system/load`
    pub async fn system_load(&self) -> Result<SystemLoad, ClientError> {
        self.get_json("/api/system/load").await
    }

    /// Orchestrate — send a query to the agent system.
    ///
    /// `POST /api/orchestrate`
    pub async fn orchestrate(&self, conversation_id: i64, query: &str) -> Result<String, ClientError> {
        #[derive(serde::Serialize)]
        struct Body<'a> { conversation_id: i64, query: &'a str }
        self.post_json("/api/orchestrate", &Body { conversation_id, query }).await
    }

    /// Get available models.
    ///
    /// `GET /api/models/available`
    pub async fn available_models(&self) -> Result<Vec<String>, ClientError> {
        self.get_json("/api/models/available").await
    }

    /// Refresh the runtime registry.
    ///
    /// `POST /api/registry/refresh`
    pub async fn refresh_registry(&self) -> Result<(), ClientError> {
        self.post_void("/api/registry/refresh", &serde_json::json!({})).await
    }

    /// Get registry status.
    ///
    /// `GET /api/registry/status`
    pub async fn registry_status(&self) -> Result<RuntimeRegistryStatus, ClientError> {
        self.get_json("/api/registry/status").await
    }
}

/// URL-encode a string for use in query parameters.
fn urlencoding(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}
