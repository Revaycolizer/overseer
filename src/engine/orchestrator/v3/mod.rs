pub mod state;
pub mod atlas;
pub mod lattice;
pub mod swarm;

pub use state::{CellState, GlobalState};
pub use atlas::{AgentAtlas, search_atlas};
pub use lattice::{LatticeCoordinator, SwarmScheduler};
pub use swarm::{calculate_consensus, SwarmVerifier};
