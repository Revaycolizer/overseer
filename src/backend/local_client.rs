pub mod agents;
pub mod conversations;
pub mod messages;
pub mod misc;
pub mod planners;
pub mod work_items;
pub mod workflows;

use crate::backend::client::BackendClient;
use crate::db::Database;
use crate::engine::system::AgentSystem;
use std::sync::Arc;

pub struct LocalClient {
    pub db: Arc<Database>,
    pub agent_system: Arc<AgentSystem>,
    pub browser_pool: Arc<crate::engine::browser::pool::BrowserPool>,
}

impl LocalClient {
    pub fn new(db: Arc<Database>, agent_system: Arc<AgentSystem>) -> Self {
        let data_dir = crate::db::get_overseer_data_dir();
        let browser_pool = Arc::new(crate::engine::browser::pool::BrowserPool::new(
            crate::engine::browser::pool_config::PoolConfig::default(),
            data_dir,
        ));
        Self {
            db,
            agent_system,
            browser_pool,
        }
    }
}

impl BackendClient for LocalClient {}
