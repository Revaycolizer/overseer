pub mod indexer;
pub mod search;

pub use indexer::AgentAtlas;
pub use search::{group_into_cells, search_atlas};
