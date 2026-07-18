#![recursion_limit = "512"]

use dioxus::prelude::*;

pub mod backend;
#[path = "db/mod.rs"]
pub mod db;
pub mod engine;
pub mod routes;
pub mod state;
pub mod server;
pub mod tui;

use routes::{
    AgentsWorkshop, BrowserSessions, ComputerControl, Conversations, DeviceControl, Layout,
    PlannersWorkshop, Settings, SupervisorsWorkshop, ToolsWorkshop, WorkflowAutomation, Listening,
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
            #[route("/browser")]
            BrowserSessions {},
            #[route("/settings/voice-training")]
            VoiceTraining {},
        #[end_layout]
        #[route("/listening")]
        Listening {},
        #[route("/idle")]
        SystemIdleSecureLockScreen {},
}
