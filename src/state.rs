use crate::db::Database;
use dioxus::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
pub struct GlobalState {
    pub agent_models: HashMap<String, String>,
    pub db: Arc<Database>,
    pub agent_system: Arc<crate::engine::system::AgentSystem>,
    pub last_activity: Signal<u64>,
    pub is_inhibited: Signal<bool>,
    pub is_listening: Signal<bool>,
    pub voice_enabled: Signal<bool>,
    pub has_trained_voice: Signal<bool>,
    pub voice_transcript: Signal<Option<String>>,
    pub voice_result: Signal<Option<String>>,
}

#[component]
pub fn StateProvider(database: Arc<Database>, children: Element) -> Element {
    let last_activity = use_signal(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
    let is_inhibited = use_signal(|| false);
    let is_listening = use_signal(|| false);
    let voice_enabled = use_signal(|| true);
    let has_trained_voice = use_signal(|| false);
    let voice_transcript = use_signal(|| None);
    let voice_result = use_signal(|| None);
    
    let state = use_signal(move || GlobalState {
        agent_models: HashMap::new(),
        db: database.clone(),
        agent_system: Arc::new(crate::engine::system::AgentSystem::new((*database).clone())),
        last_activity,
        is_inhibited,
        is_listening,
        voice_enabled,
        has_trained_voice,
        voice_transcript,
        voice_result,
    });

    use_context_provider(|| state);

    children
}

pub fn use_global_state() -> Signal<GlobalState> {
    use_context::<Signal<GlobalState>>()
}
