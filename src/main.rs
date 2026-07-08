#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dioxus::prelude::*;
mod backend;
#[path = "db/mod.rs"]
pub mod db;
pub mod engine;
mod routes;
mod state;
mod server;

use routes::{
    AgentsWorkshop, App, ComputerControl, Conversations, DeviceControl, Layout, PlannersWorkshop,
    Settings, SupervisorsWorkshop, ToolsWorkshop, WorkflowAutomation, Listening,
    SystemIdleSecureLockScreen, IdleWatcher, VoiceTraining
};

#[derive(Debug, Clone, Routable, PartialEq)]
pub enum Route {
    #[layout(IdleWatcher)]
        #[layout(Layout)]
            #[route("/")]
            Conversations {},
            #[route("/agents")]
            AgentsWorkshop {},
            #[route("/workflows")]
            WorkflowAutomation {},
            #[route("/tools")]
            ToolsWorkshop {},
            #[route("/computer")]
            ComputerControl {},
            #[route("/device")]
            DeviceControl {},
            #[route("/supervisors")]
            SupervisorsWorkshop {},
            #[route("/planners")]
            PlannersWorkshop {},
            #[route("/settings")]
            Settings {},
            #[route("/settings/voice-training")]
            VoiceTraining {},
        #[end_layout]
        #[route("/listening")]
        Listening {},
        #[route("/idle")]
        SystemIdleSecureLockScreen {},
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("📍 Overseer launched with args: {:?}", args);

    if args.len() > 1 && (args[1] == "server" || args[1] == "--server") {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            println!("🛠️  Initializing Database for Server...");
            let db = crate::db::Database::new().await.unwrap();
            let agent_system = std::sync::Arc::new(crate::engine::system::AgentSystem::new(db.clone()));
            if let Err(e) = crate::server::run_server(std::sync::Arc::new(db), agent_system, 8080).await {
                eprintln!("Server Error: {}", e);
            }
        });
        return;
    }

    if args.len() > 1 && (args[1] == "info" || args[1] == "--info" || !args[1].starts_with("-")) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        if let Err(e) = rt.block_on(engine::shell::run_cli()) {
            eprintln!("Error: {}", e);
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
