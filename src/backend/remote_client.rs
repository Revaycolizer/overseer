pub mod agents;
pub mod conversations;
pub mod messages;
pub mod misc;
pub mod planners;
pub mod work_items;
pub mod workflows;

use crate::backend::client::BackendClient;

pub struct RemoteClient {
    pub base_url: String,
    pub http_client: reqwest::Client,
}

impl RemoteClient {
    pub fn new(base_url: String) -> Self {
        Self {
            base_url,
            http_client: {
                let mut headers = reqwest::header::HeaderMap::new();
                if let Ok(token) = std::env::var("OVERSEER_API_TOKEN") {
                    if let Ok(mut value) =
                        reqwest::header::HeaderValue::from_str(&format!("Bearer {token}"))
                    {
                        value.set_sensitive(true);
                        headers.insert(reqwest::header::AUTHORIZATION, value);
                    }
                }
                reqwest::Client::builder()
                    .default_headers(headers)
                    .timeout(std::time::Duration::from_secs(120))
                    .build()
                    .expect("HTTP client configuration")
            },
        }
    }
}

impl BackendClient for RemoteClient {}
