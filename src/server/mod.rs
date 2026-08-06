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

    // Background services (run inside the resident daemon).
    // Scheduler: always on — it just polls an (initially empty) table.
    {
        let sched_db = db.clone();
        tokio::spawn(async move { crate::engine::scheduler::run_scheduler(sched_db).await });
    }
    // Voice: opt-in — audio capture needs a real input device.
    let voice_on = crate::backend::get_setting_or(&db, "voice_always_on", "false").await;
    if voice_on == "true" || voice_on == "1" {
        let vdb = db.clone();
        let vagent = agent_system.clone();
        tokio::spawn(async move { crate::engine::voice_service::run_voice_service(vdb, vagent).await });
    } else {
        tracing::info!("voice service disabled (set setting 'voice_always_on' = true to enable)");
    }

    let state = Arc::new(ServerState { db, agent_system, browser_pool: browser_pool.clone() });

    let app = Router::new()
        .merge(conversations::router())
        .merge(messages::router())
        .merge(agents::router())
        .merge(supervisors::router())
        .merge(tools::router())
        .merge(workflows::router())
        .merge(work_items::router())
        .merge(workflow_triggers::router())
        .merge(planners::router())
        .merge(workflow_triggers::router())
        .merge(supervision::router())
        .merge(versions::router())
        .merge(artifacts::router())
        .merge(misc::router())
        .merge(browser::router())
        .merge(auth::router())
        .merge(browser_recording::router())
        .merge(browser_hitl::router())
        .merge(credentials::router())
        .merge(workflow_templates::router())
        .merge(workflow_triggers::router())
        .merge(crate::mcp::server::router())
        .merge(crate::server::metrics::router())
        .merge(crate::server::openapi::router())
        .merge(ws::router())
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
pub mod workflow_triggers;
pub mod workflows;
pub mod work_items;
pub mod planners;
pub mod supervision;
pub mod versions;
pub mod artifacts;
pub mod auth;
pub mod browser;
pub mod workflow_templates;
pub mod credentials;
pub mod browser_hitl;
pub mod browser_recording;
pub mod browser_relay;
pub mod metrics;
pub mod misc;
pub mod openapi;
pub mod ws;
