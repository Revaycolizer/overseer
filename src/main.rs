#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dioxus::prelude::*;
mod backend;
#[path = "db/mod.rs"]
pub mod db;
pub mod engine;
mod routes;
mod state;

use routes::{
    AgentsWorkshop, App, ComputerControl, Conversations, Layout, PlannersWorkshop, Settings,
    SupervisorsWorkshop, ToolsWorkshop, WorkflowAutomation, Listening, SystemIdleSecureLockScreen,
    IdleWatcher
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
            #[route("/supervisors")]
            SupervisorsWorkshop {},
            #[route("/planners")]
            PlannersWorkshop {},
            #[route("/settings")]
            Settings {},
        #[end_layout]
        #[route("/listening")]
        Listening {},
        #[route("/idle")]
        SystemIdleSecureLockScreen {},
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    println!("📍 Overseer launched with args: {:?}", args);

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
