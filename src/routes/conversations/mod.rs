pub mod chat_input;
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

pub use conversations::Conversations;
