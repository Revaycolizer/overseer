pub mod conversations;
pub mod messages;
pub mod agents;
pub mod workflows;
pub mod work_items;
pub mod planners;
pub mod misc;

use crate::backend::client::BackendClient;
use std::sync::Arc;

pub struct RemoteClient {
    pub base_url: String,
    pub http_client: reqwest::Client,
}

impl RemoteClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http_client: reqwest::Client::new(),
        }
    }
}

impl BackendClient for RemoteClient {}
