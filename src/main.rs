#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tracing_subscriber::EnvFilter;

use overseer::routes::App;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args: Vec<String> = std::env::args().collect();
    tracing::info!(?args, "Overseer launched");

    if args.len() > 1 && (args[1] == "server" || args[1] == "--server") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            tracing::info!("Initializing Database for Server...");
            let db = overseer::db::Database::new().await.unwrap();
            let agent_system = std::sync::Arc::new(overseer::engine::system::AgentSystem::new(db.clone()));
            if let Err(e) = overseer::server::run_server(std::sync::Arc::new(db), agent_system, 8080).await {
                tracing::error!(?e, "Server error");
            }
        });
        return;
    }

    if args.len() > 1 && (args[1] == "info" || args[1] == "--info" || !args[1].starts_with("-")) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Err(e) = rt.block_on(overseer::engine::shell::run_cli()) {
            tracing::error!(?e, "CLI error");
        }
        return;
    }

    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, WindowBuilder};
        let config = Config::new().with_window(
            WindowBuilder::new()
                .with_title("Overseer")
                .with_decorations(false),
        );
        dioxus::LaunchBuilder::desktop()
            .with_cfg(config)
            .launch(App);
    }
    #[cfg(not(feature = "desktop"))]
    dioxus::launch(App);
}
