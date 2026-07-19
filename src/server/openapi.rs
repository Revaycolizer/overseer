//! API documentation endpoints.
//!
//! Serves the OpenAPI 3.0 specification at `/openapi.json` using utoipa
//! and the Swagger UI at `/docs`.
//!
//! Route handlers are annotated with `#[utoipa::path(...)]` in each
//! server module. As modules are annotated, add them to the `paths(...)`
//! list in `ApiDoc` below and their types to `components(schemas(...))`.

use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::server::ServerState;
use axum::Router;
use std::sync::Arc;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Overseer REST API",
        description = "Local-first AI orchestration platform — conversations, agents, tools, workflows, and more.\n\nSee https://github.com/revaycolizer/overseer for the full API reference.",
        version = env!("CARGO_PKG_VERSION"),
        license(name = "MIT", url = "https://github.com/revaycolizer/overseer/blob/main/LICENSE"),
    ),
    servers(
        (url = "/", description = "Server root"),
    ),
    tags(
        (name = "Auth", description = "User authentication and session management"),
        (name = "Conversations", description = "Conversation CRUD and search"),
        (name = "Messages", description = "Message and attachment management"),
        (name = "Agents", description = "Agent discovery and management"),
        (name = "Tools", description = "Tool registration and agent linking"),
        (name = "Supervisors", description = "Supervisor management"),
        (name = "Workflows", description = "Workflow definitions, steps, and runs"),
        (name = "Work Items", description = "Work item (task) management"),
        (name = "Artifacts", description = "Artifact storage and retrieval"),
        (name = "Planners", description = "Planner configuration"),
        (name = "Supervision", description = "Computer supervision and kernel approvals"),
        (name = "Version Control", description = "Entity version publishing and rollback"),
        (name = "Browser", description = "Browser session management"),
        (name = "Credentials", description = "Credential storage"),
        (name = "Workflow Templates", description = "Workflow template management"),
        (name = "Settings", description = "Key-value settings"),
        (name = "System", description = "Health, metrics, and system information"),
    ),
    paths(
        // Annotated endpoints go here as modules are documented.
        // See server/auth.rs, server/conversations.rs etc. for examples.
    ),
    components(
        schemas(
            // Annotated types go here as modules are documented.
        ),
    ),
)]
pub struct ApiDoc;

/// Build the Swagger UI router, served at `/docs` and `/openapi.json`.
pub fn router() -> Router<Arc<ServerState>> {
    SwaggerUi::new("/docs")
        .url("/openapi.json", ApiDoc::openapi())
        .into()
}
