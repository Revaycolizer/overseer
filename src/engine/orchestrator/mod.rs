pub mod discovery;
pub mod result;
pub mod response;
pub mod formatter;
pub mod format_helpers;
mod dispatcher;
pub mod dynamic;
pub mod grounding;
pub mod guardrails;
pub mod master;
mod planner;
pub mod native_checks;
pub mod native_render;
pub mod native_shortcut;
pub mod artifact_utils;
pub mod event_logger;
pub mod context_helpers;
pub mod errors;
pub mod decisions;
pub mod main_loop;
pub mod direct_chat;
pub mod executor;
pub mod planner_utils;

pub use dynamic::{ChatAgent, OverseerAgent, VisionGeneralAgent};
pub use master::MasterPlanner;
pub use planner::MacroTask;
pub use main_loop::orchestrate_request;
pub use native_shortcut::try_native_computer_shortcut;

use native_checks::*;
use native_render::*;
use artifact_utils::*;
use event_logger::*;
use context_helpers::*;
use errors::*;
use decisions::*;

use rig::{
    client::CompletionClient,
    completion::{Message, Prompt},
    providers::ollama,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{sleep, timeout};

use crate::engine::tools::StructuredPerceptionTool;
use crate::engine::utils::discovery::{AgentInfo, ToolInfo};

// Orchestration Logic Modularization - Overseer Core Orchestrator
// All primary functions moved to specialized sub-modules for maintainability and scalability.
