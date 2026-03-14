use dioxus::prelude::*;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use crate::db::Database;
use std::sync::Arc;

#[derive(Clone)]
pub struct GlobalState {
    pub agent_models: HashMap<String, String>,
    pub db: Arc<Database>,
    pub agent_system: Arc<crate::engine::system::AgentSystem>,
}

#[component]
pub fn StateProvider(database: Arc<Database>, children: Element) -> Element {
    let state = use_signal(move || GlobalState {
        agent_models: HashMap::new(),
        db: database.clone(),
        agent_system: Arc::new(crate::engine::system::AgentSystem::new((*database).clone())),
    });

    use_context_provider(|| state);

    children
}

pub fn use_global_state() -> Signal<GlobalState> {
    use_context::<Signal<GlobalState>>()
}
