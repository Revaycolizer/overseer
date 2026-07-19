#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tracing_subscriber::EnvFilter;

use overseer::routes::App;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn")),
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

    if args.len() > 1 && (args[1] == "backup" || args[1] == "--backup") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match overseer::db::Database::new().await {
                Ok(db) => {
                    match overseer::engine::backup::create_backup(&db, None).await {
                        Ok(path) => println!("✅ Backup created: {}", path.display()),
                        Err(e) => eprintln!("❌ Backup failed: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Database init failed: {}", e),
            }
        });
        return;
    }

    if args.len() > 1 && (args[1] == "export" || args[1] == "--export") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match overseer::db::Database::new().await {
                Ok(db) => {
                    match overseer::engine::backup::export_all_conversations(&db).await {
                        Ok(json) => println!("{}", json),
                        Err(e) => eprintln!("❌ Export failed: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Database init failed: {}", e),
            }
        });
        return;
    }

    if args.len() > 1 && (args[1] == "migrate-check" || args[1] == "--migrate-check") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            match overseer::db::Database::new().await {
                Ok(db) => {
                    match overseer::engine::backup::migration_dry_run(&db).await {
                        Ok(pending) => {
                            if pending.is_empty() {
                                println!("✅ All migrations applied.");
                            } else {
                                println!("📋 Pending migrations:");
                                for m in pending { println!("   {}", m); }
                            }
                        }
                        Err(e) => eprintln!("❌ Check failed: {}", e),
                    }
                }
                Err(e) => eprintln!("❌ Database init failed: {}", e),
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
