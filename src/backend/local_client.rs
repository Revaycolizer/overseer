pub mod conversations;
pub mod messages;
pub mod agents;
pub mod workflows;
pub mod work_items;
pub mod planners;
pub mod misc;

use crate::backend::client::BackendClient;
use crate::db::Database;
use crate::engine::system::AgentSystem;
use std::sync::Arc;

pub struct LocalClient {
    pub db: Arc<Database>,
    pub agent_system: Arc<AgentSystem>,
}

impl LocalClient {
    pub fn new(db: Arc<Database>, agent_system: Arc<AgentSystem>) -> Self {
        Self { db, agent_system }
    }
}

impl BackendClient for LocalClient {}
