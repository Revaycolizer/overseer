use axum::Router;
use crate::db::Database;
use crate::engine::system::AgentSystem;
use std::sync::Arc;
use std::net::SocketAddr;
use tokio::signal;
use tower_governor::GovernorLayer;

pub struct ServerState {
    pub db: Arc<Database>,
    pub agent_system: Arc<AgentSystem>,
    pub browser_pool: Arc<crate::engine::browser::pool::BrowserPool>,
}

pub async fn run_server(db: Arc<Database>, agent_system: Arc<AgentSystem>, port: u16) -> anyhow::Result<()> {
    let data_dir = crate::db::get_overseer_data_dir();
    let pool = crate::engine::browser::pool::BrowserPool::new(
        crate::engine::browser::pool_config::PoolConfig::default(),
        data_dir,
    );
    let browser_pool = Arc::new(pool);
    crate::engine::browser::global::init(browser_pool.clone());
    let state = Arc::new(ServerState { db, agent_system, browser_pool: browser_pool.clone() });

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
        .merge(browser::router())
        .merge(auth::router())
        .merge(browser_recording::router())
        .merge(browser_hitl::router())
        .layer(GovernorLayer {
            config: crate::engine::resilience::server_governor(),
        })
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    println!("🚀 Overseer Server running on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;

    let shutdown = {
        let bp = browser_pool.clone();
        async move {
            signal::ctrl_c().await.ok();
            tracing::info!("Shutting down browser sessions...");
            bp.shutdown().await;
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown)
        .await?;
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
pub mod auth;
pub mod browser;
pub mod browser_hitl;
pub mod browser_recording;
pub mod browser_relay;
pub mod misc;
