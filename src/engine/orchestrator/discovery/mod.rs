// src/engine/orchestrator/discovery/mod.rs
pub mod dynamic;
pub mod fast_path;
pub mod grounding;
pub mod rerank;
pub mod semantic;
pub mod utils;

pub use fast_path::discover_fast_path_domain;
pub use semantic::discover_semantic_agents;
pub use utils::looks_like_planner_request;
