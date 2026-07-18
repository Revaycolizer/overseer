pub mod assistant_stream;
pub mod chat_input;
pub mod chat_panel;
mod conversations;
pub mod helpers;
pub mod message_bubble;
pub mod sidebar;

// New modularized components
pub mod markdown_renderer;
pub mod attachment_view;
pub mod attachment_meta;
pub mod search_view;
pub mod chart_view;
pub mod types;
pub mod structured_view;
pub mod thinking_bubble;
pub mod legacy_parser;
pub mod content_cleaner;
pub mod clipboard;
pub mod query_builder;
pub mod message_parser;

// Architecture V3.9 — decomposed from conversations.rs
pub mod work_item_ops;
pub mod work_item_runner;
pub mod work_item_runner_finalize;
pub mod work_item_handoff;
pub mod workspace_session;
pub mod route_summary;
pub mod registry_panel;
pub mod empty_state;
pub mod artifact_panel;
pub mod artifact_card;
pub mod artifact_workbench;
pub mod artifact_workbench_parts;
pub mod artifact_workbench_utils;
pub mod workboard_panel;
pub mod work_item_card;
pub mod work_item_card_parts;
pub mod workspace_deck;
pub mod work_item_panel;
pub mod work_item_panel_parts;
pub mod work_item_panel_extras;

pub mod state;
pub mod hooks;
pub mod prefetch;
pub mod chip;
pub mod layout;
pub mod content;

pub use conversations::Conversations;
