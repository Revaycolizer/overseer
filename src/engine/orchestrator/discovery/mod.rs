// src/engine/orchestrator/discovery/mod.rs
pub mod fast_path;
pub mod semantic;
pub mod rerank;
pub mod utils;
pub mod grounding;
pub mod dynamic;

pub use fast_path::discover_fast_path_domain;
pub use semantic::discover_semantic_agents;
pub use utils::looks_like_planner_request;
