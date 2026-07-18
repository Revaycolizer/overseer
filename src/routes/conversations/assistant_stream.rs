use dioxus::prelude::*;
use tokio::sync::oneshot;

use crate::backend::Message;

/// Wire up the assistant-message channel: returns a `Sender` that the
/// non-`Send` `tokio::spawn` future inside `ChatInput` uses to publish
/// the freshly-completed assistant message; a `Dioxus spawn` listens on
/// the receiver and pushes it into `visible_messages`.
pub fn install_listener(mut visible_messages: Signal<Vec<Message>>) -> oneshot::Sender<Message> {
    let (tx, rx) = oneshot::channel::<Message>();
    spawn(async move {
        if let Ok(msg) = rx.await {
            visible_messages.with_mut(|m| m.push(msg));
        }
    });
    tx
}

/// Build a `Message` value for an in-progress assistant response.
/// The DB persists via `backend::add_message` separately; this is
/// for the in-stream optimistic chat list.
pub fn assistant_message(conversation_id: i64, response: String) -> Message {
    Message {
        id: 0,
        conversation_id,
        role: "assistant".to_string(),
        content: response,
        created_at: chrono::Utc::now().naive_utc(),
        attachments: Vec::new(),
        hide_evidence: false,
    }
}