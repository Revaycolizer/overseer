pub mod core;
pub mod planning;
pub mod discovery;
pub mod utils;
pub mod native;
pub mod guardrails;

// Architecture V3: Top-level components
pub mod lattice;
pub mod swarm;
pub mod state;
pub mod atlas;

// Re-exports for backward compatibility and convenience
pub use core::main_loop::orchestrate_request;
pub use core::result::SupervisorExecutionResult;
pub use core::direct_chat;
pub use core::response::{StructuredResponse, MessagePart, SourcePart, ChartPart, UnknownPart};
pub use planning::master::MasterPlanner;
pub use planning::planner::MacroTask;
pub use crate::engine::utils::discovery::{AgentInfo, ToolInfo};
pub use discovery::dynamic::{ChatAgent, OverseerAgent, VisionGeneralAgent};
pub use native::native_shortcut::try_native_computer_shortcut;

// Re-map legacy dispatch to the new Swarm/Lattice structure where appropriate
// For now, we keep the dispatch folder but it will be moved into swarm/atlas later.
pub mod dispatch;
pub use dispatch::dispatch_domain;

// Specific commonly used types
pub use planning::decisions::{OrchestrationDecision, OrchestrationRoute};
pub use core::errors::OrchestratorError;

use rig::{
    client::CompletionClient,
    completion::{Message, Prompt},
    providers::ollama,
};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::{sleep, timeout};

use crate::engine::tools::StructuredPerceptionTool;

pub mod r#loop;
pub use r#loop::{AgentOrchestrator, Action};
