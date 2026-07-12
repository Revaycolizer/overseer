use axum::Router;
use axum::body::Body;
use axum::http::Request;
use axum::middleware::{self, Next};
use axum::response::Response;
use crate::db::Database;
use crate::engine::system::AgentSystem;
use std::sync::Arc;
use std::net::SocketAddr;

pub struct ServerState {
    pub db: Arc<Database>,
    pub agent_system: Arc<AgentSystem>,
}

async fn rate_limit_middleware(req: Request<Body>, next: Next) -> Response {
    let _permit = crate::engine::resilience::server_throttle().acquire().await;
    next.run(req).await
}

pub async fn run_server(db: Arc<Database>, agent_system: Arc<AgentSystem>, port: u16) -> anyhow::Result<()> {
    let state = Arc::new(ServerState { db, agent_system });

    let app = Router::new()
        .merge(conversations::router())
        .merge(messages::router())
        .merge(agents::router())
        .merge(supervisors::router())
        .merge(tools::router())
        .merge(workflows::router())
        .merge(work_items::router())
        .merge(planners::router())
        .merge(supervision::router())
        .merge(versions::router())
        .merge(artifacts::router())
        .merge(misc::router())
        .layer(middleware::from_fn(rate_limit_middleware))
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("🚀 Overseer Server running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

pub mod conversations;
pub mod messages;
pub mod agents;
pub mod supervisors;
pub mod tools;
pub mod workflows;
pub mod work_items;
pub mod planners;
pub mod supervision;
pub mod versions;
pub mod artifacts;
pub mod misc;
