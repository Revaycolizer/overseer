pub mod core;
pub mod discovery;
pub mod guardrails;
pub mod native;
pub mod planning;
pub mod truthfulness;
pub mod utils;

// Architecture V3: Top-level components
pub mod atlas;
pub mod lattice;
pub mod state;
pub mod swarm;

// Re-exports for backward compatibility and convenience
pub use crate::engine::utils::discovery::{AgentInfo, ToolInfo};
pub use core::direct_chat;
pub use core::main_loop::orchestrate_request;
pub use core::response::{ChartPart, MessagePart, SourcePart, StructuredResponse, UnknownPart};
pub use core::result::SupervisorExecutionResult;
pub use discovery::dynamic::{ChatAgent, OverseerAgent, VisionGeneralAgent};
pub use native::native_shortcut::try_native_computer_shortcut;
pub use planning::master::MasterPlanner;
pub use planning::planner::MacroTask;

// Re-map legacy dispatch to the new Swarm/Lattice structure where appropriate
// For now, we keep the dispatch folder but it will be moved into swarm/atlas later.
pub mod dispatch;
pub use dispatch::dispatch_domain;

// Specific commonly used types
pub use core::errors::OrchestratorError;
pub use planning::decisions::{OrchestrationDecision, OrchestrationRoute};

pub mod r#loop;
pub mod memory_compactor;
pub use r#loop::{Action, AgentOrchestrator};
