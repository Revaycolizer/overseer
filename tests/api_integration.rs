//! Integration tests for the Overseer REST API.
//!
//! Run: cargo test --test api_integration

use std::sync::Arc;

use axum::Router;
use serde_json::Value;

use overseer::db::Database;
use overseer::server::ServerState;

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

struct TestApp {
    client: reqwest::Client,
    base_url: String,
}

impl TestApp {
    async fn new() -> Self {
        let db = Arc::new(Database::new_in_memory().await.unwrap());

        let agent_system = Arc::new(overseer::engine::system::AgentSystem {
            ollama_client: rig::providers::ollama::Client::new(rig::client::Nothing)
                .expect("test AgentSystem should always succeed"),
        });

        let browser_pool = Arc::new(overseer::engine::browser::pool::BrowserPool::new(
            overseer::engine::browser::pool_config::PoolConfig::default(),
            std::env::temp_dir(),
        ));

        let state = Arc::new(ServerState { db, agent_system, browser_pool });

        let app: Router = Router::new()
            .merge(overseer::server::conversations::router())
            .merge(overseer::server::messages::router())
            .merge(overseer::server::agents::router())
            .merge(overseer::server::tools::router())
            .merge(overseer::server::supervisors::router())
            .merge(overseer::server::workflows::router())
            .merge(overseer::server::work_items::router())
            .merge(overseer::server::artifacts::router())
            .merge(overseer::server::planners::router())
            .merge(overseer::server::supervision::router())
            .merge(overseer::server::versions::router())
            .merge(overseer::server::misc::router())
            .merge(overseer::server::auth::router())
            .with_state(state);

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        Self { client: reqwest::Client::new(), base_url }
    }

    async fn get(&self, path: &str) -> (u16, Option<Value>) {
        let r = self.client.get(&format!("{}{}", self.base_url, path))
            .send().await.unwrap();
        let status = r.status().as_u16();
        let body = r.json().await.ok();
        (status, body)
    }

    async fn post(&self, path: &str, body: &Value) -> (u16, Option<Value>) {
        let r = self.client.post(&format!("{}{}", self.base_url, path))
            .json(body).send().await.unwrap();
        let status = r.status().as_u16();
        let body = r.json().await.ok();
        (status, body)
    }

    /// Post, expect 200, return deserialized body.
    async fn post_ok(&self, path: &str, body: &Value) -> Value {
        let (status, body) = self.post(path, body).await;
        assert_eq!(status, 200, "POST {path} failed");
        body.unwrap_or(Value::Null)
    }

    /// Get, expect 200, return deserialized body.
    async fn get_ok(&self, path: &str) -> Value {
        let (status, body) = self.get(path).await;
        assert_eq!(status, 200, "GET {path} failed");
        body.unwrap_or(Value::Null)
    }
}

// ---------------------------------------------------------------------------
// Health & System
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_health() {
    let app = TestApp::new().await;
    let body = app.get_ok("/health").await;
    // Health returns system info — at minimum should have some fields
    assert!(body.is_object());
    assert!(!body.as_object().unwrap().is_empty());
}

#[tokio::test]
async fn test_settings() {
    let app = TestApp::new().await;

    app.post_ok("/api/settings", &serde_json::json!({"key": "k", "value": "v"})).await;
    let body = app.get_ok("/api/settings?key=k").await;
    assert_eq!(body, "v");

    let all = app.get_ok("/api/settings/all").await;
    assert!(all.is_object());
}

#[tokio::test]
async fn test_system_load() {
    let app = TestApp::new().await;
    let body = app.get_ok("/api/system/load").await;
    // System load returns CPU/memory info — just check it's a valid object
    assert!(body.is_object());
}

// ---------------------------------------------------------------------------
// Conversations
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_conversations_crud() {
    let app = TestApp::new().await;

    // Create
    let body = app.post_ok("/api/conversations", &serde_json::json!({"title": "Test"})).await;
    let conv_id: i64 = serde_json::from_value(body).unwrap();

    // List — should contain ours
    let list = app.get_ok("/api/conversations?limit=10").await;
    let convs = list.as_array().unwrap();
    assert!(convs.iter().any(|c| c["id"] == conv_id));
    assert_eq!(convs[0]["title"], "Test");

    // Update title
    app.post_ok("/api/conversations/update_title", &serde_json::json!({"id": conv_id, "title": "Updated"})).await;

    // Delete
    app.post_ok("/api/conversations/delete", &serde_json::json!({"id": conv_id})).await;
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_messages_crud() {
    let app = TestApp::new().await;

    let body = app.post_ok("/api/conversations", &serde_json::json!({"title": "MsgTest"})).await;
    let conv_id: i64 = serde_json::from_value(body).unwrap();

    // List — empty
    let list = app.get_ok(&format!("/api/messages?conversation_id={conv_id}")).await;
    assert!(list.as_array().unwrap().is_empty());

    // Send
    let body = app.post_ok("/api/messages", &serde_json::json!({
        "conversation_id": conv_id, "role": "user", "content": "Hello!", "hide_evidence": false,
    })).await;
    let msg_id: i64 = serde_json::from_value(body).unwrap();

    // List — one message
    let list = app.get_ok(&format!("/api/messages?conversation_id={conv_id}")).await;
    let msgs = list.as_array().unwrap();
    assert_eq!(msgs.len(), 1);
    assert_eq!(msgs[0]["role"], "user");
    assert_eq!(msgs[0]["content"], "Hello!");

    // Delete
    app.post_ok("/api/messages/delete", &serde_json::json!({"id": msg_id})).await;
}

// ---------------------------------------------------------------------------
// Agents
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_agents_create() {
    let app = TestApp::new().await;
    let body = app.post_ok("/api/agents", &serde_json::json!({
        "name": "test-agent", "description": "Test", "domain": "testing",
    })).await;
    let id: i64 = serde_json::from_value(body).unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn test_agents_list() {
    let app = TestApp::new().await;
    // Create an agent (in-memory DB has seeded agents too)
    app.post_ok("/api/agents", &serde_json::json!({
        "name": "unique-agent", "description": "Test", "domain": "uniq",
    })).await;
    let list = app.get_ok("/api/agents").await;
    let agents = list.as_array().unwrap();
    assert!(!agents.is_empty(), "Should have at least our created agent");
    assert!(agents.iter().any(|a| a["name"] == "unique-agent"), "Our agent should be in the list");
}

// ---------------------------------------------------------------------------
// Tools
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_tools_create() {
    let app = TestApp::new().await;
    let body = app.post_ok("/api/tools", &serde_json::json!({
        "name": "test-tool", "description": "Test", "language": "python",
    })).await;
    let id: i64 = serde_json::from_value(body).unwrap();
    assert!(id > 0);
}

#[tokio::test]
async fn test_tools_list() {
    let app = TestApp::new().await;
    // Create a tool (in-memory DB has seeded tools too)
    app.post_ok("/api/tools", &serde_json::json!({
        "name": "unique-tool", "description": "Test", "language": "rust",
    })).await;
    let list = app.get_ok("/api/tools").await;
    let tools = list.as_array().unwrap();
    assert!(!tools.is_empty(), "Should have at least our created tool");
    assert!(tools.iter().any(|t| t["name"] == "unique-tool"), "Our tool should be in the list");
}

// ---------------------------------------------------------------------------
// Supervisors
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_supervisors_crud() {
    let app = TestApp::new().await;

    // List (empty)
    let _ = app.get_ok("/api/supervisors").await;

    // Create
    let body = app.post_ok("/api/supervisors", &serde_json::json!({
        "name": "Test Sup", "description": "Supervises", "domain": "testing",
    })).await;
    let sup_id: i64 = serde_json::from_value(body).unwrap();

    // Delete
    app.post_ok("/api/supervisors/delete", &serde_json::json!({"id": sup_id})).await;
}

// ---------------------------------------------------------------------------
// Workflows
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_workflows_crud() {
    let app = TestApp::new().await;

    let body = app.post_ok("/api/workflows", &serde_json::json!({"name": "Test WF"})).await;
    let wf_id: i64 = serde_json::from_value(body).unwrap();

    // Get steps (empty)
    let steps = app.get_ok(&format!("/api/workflows/{wf_id}/steps")).await;
    assert!(steps.as_array().unwrap().is_empty());

    app.post_ok("/api/workflows/delete", &serde_json::json!({"id": wf_id})).await;
}

// ---------------------------------------------------------------------------
// Work Items
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_work_items() {
    let app = TestApp::new().await;

    let body = app.post_ok("/api/conversations", &serde_json::json!({"title": "WI"})).await;
    let conv_id: i64 = serde_json::from_value(body).unwrap();

    // List — empty
    let list = app.get_ok(&format!("/api/work_items?conversation_id={conv_id}")).await;
    assert!(list.as_array().unwrap().is_empty());

    // Create
    let body = app.post_ok("/api/work_items", &serde_json::json!({
        "conversation_id": conv_id, "title": "Test WI", "status": "queued", "prompt": "Do it",
    })).await;
    let wi_id: i64 = serde_json::from_value(body).unwrap();
    assert!(wi_id > 0);
}

// ---------------------------------------------------------------------------
// Planners
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_planners() {
    let app = TestApp::new().await;

    let body = app.post_ok("/api/planners", &serde_json::json!({
        "name": "test-pln", "instructions": "Do", "model": "qwen2.5:7b",
    })).await;
    let id: i64 = serde_json::from_value(body).unwrap();
    assert!(id > 0);
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_auth_register() {
    let app = TestApp::new().await;
    let body = app.post_ok("/v1/auth/register", &serde_json::json!({
        "username": "user1", "display_name": "User 1", "password": "pass",
    })).await;
    assert!(body["token"].is_string());
    assert_eq!(body["user"]["username"], "user1");
}

#[tokio::test]
async fn test_auth_login() {
    let app = TestApp::new().await;

    app.post_ok("/v1/auth/register", &serde_json::json!({
        "username": "user2", "display_name": "User 2", "password": "pass",
    })).await;

    let body = app.post_ok("/v1/auth/login", &serde_json::json!({
        "username": "user2", "password": "pass",
    })).await;
    assert!(body["token"].is_string());
    assert_eq!(body["user"]["username"], "user2");
}

#[tokio::test]
async fn test_auth_me() {
    let app = TestApp::new().await;

    let body = app.post_ok("/v1/auth/register", &serde_json::json!({
        "username": "meuser", "display_name": "Me", "password": "pass",
    })).await;
    let token = body["token"].as_str().unwrap().to_string();

    let r = app.client.get(&format!("{}/v1/auth/me", app.base_url))
        .header("Authorization", format!("Bearer {token}"))
        .send().await.unwrap();
    assert_eq!(r.status().as_u16(), 200);
    let me: Value = r.json().await.unwrap();
    assert_eq!(me["username"], "meuser");
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_registry() {
    let app = TestApp::new().await;

    let body = app.get_ok("/api/registry/status").await;
    // Registry status returns info — at minimum it should be an object
    assert!(body.is_object());

    app.post_ok("/api/registry/refresh", &serde_json::json!({})).await;
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_models() {
    let app = TestApp::new().await;
    let body = app.get_ok("/api/models/available").await;
    let models = body.as_array().unwrap();
    assert!(models.iter().all(|m| m.is_string()));
}
