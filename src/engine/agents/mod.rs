pub mod master_planner;
pub mod overseer;
pub mod chat;
pub mod stealth;
pub mod vision;

// Domain specific modules
pub mod web;
pub mod office;
pub mod desktop;
pub mod text_writer;
pub mod neontune;
pub mod browser;

pub use master_planner::MasterPlanner;
pub use web::{WebSupervisor, WebPlanner};
pub use office::{OfficeSupervisor, OfficePlanner};
pub use desktop::{AppSupervisor, DesktopPlanner, DesktopAgent};
pub use text_writer::TextWriterSupervisor;
pub use neontune::{NeontuneSupervisor, NeontunePlanner};
pub use chat::ChatAgent;
pub use stealth::StealthSupervisor;
pub use browser::BrowserAgent;
pub use overseer::OverseerAgent;
pub use vision::VisionGeneralAgent;
