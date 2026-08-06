//! WebSocket streaming endpoint for external UIs (e.g. the Tauri/React client).
//!
//! `GET /api/ws/chat` upgrades to a WebSocket. The client sends
//! `{"conversation_id": <i64>, "message": "<text>"}`; the server persists the
//! user message, runs it through the engine (governed `/write` etc. go straight
//! through the kernel, everything else through the orchestrator), streams
//! per-token `{"type":"token","text":...}` frames, then a final
//! `{"type":"done","response":...}`.
//!
//! Optional auth: if `OVERSEER_API_TOKEN` is set, the first frame must be
//! `{"token":"<value>"}` or the socket is closed. When unset, the socket is open
//! (loopback-only), matching the current local behavior.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;

use super::ServerState;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/api/ws/chat", get(ws_handler))
}

#[derive(Deserialize)]
struct ChatReq {
    conversation_id: i64,
    message: String,
}

#[derive(Deserialize)]
struct TokenFrame {
    token: String,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<ServerState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<ServerState>) {
    // Optional token gate.
    if let Ok(expected) = std::env::var("OVERSEER_API_TOKEN") {
        if !expected.is_empty() {
            let ok = match socket.recv().await {
                Some(Ok(Message::Text(t))) => serde_json::from_str::<TokenFrame>(&t)
                    .map(|f| f.token == expected)
                    .unwrap_or(false),
                _ => false,
            };
            if !ok {
                let _ = socket
                    .send(Message::Text(json!({"type":"error","error":"unauthorized"}).to_string()))
                    .await;
                return;
            }
        }
    }

    while let Some(Ok(msg)) = socket.recv().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };
        let Ok(req) = serde_json::from_str::<ChatReq>(&text) else {
            let _ = socket
                .send(Message::Text(json!({"type":"error","error":"bad request"}).to_string()))
                .await;
            continue;
        };

        let db = state.db.clone();
        let agent = state.agent_system.clone();
        let conv = req.conversation_id;
        let message = req.message.clone();

        let _ = crate::backend::add_message(&db, conv, "user", &message, false).await;

        // Run the request; stream tokens through an mpsc channel.
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
        let db_task = db.clone();
        let handle = tokio::spawn(async move {
            if let Some(cmd) = crate::engine::governed_commands::parse_governed(&message) {
                crate::engine::governed_commands::run_governed(&db_task, conv, cmd).await
            } else {
                agent.orchestrate(&db_task, conv, &message, Some(tx)).await
            }
        });

        // Forward streamed tokens to the client.
        while let Some(tok) = rx.recv().await {
            if socket
                .send(Message::Text(json!({"type":"token","text": tok}).to_string()))
                .await
                .is_err()
            {
                return; // client disconnected
            }
        }

        let response = handle.await.unwrap_or_else(|_| String::new());
        let _ = crate::backend::add_message(&db, conv, "assistant", &response, false).await;
        let _ = socket
            .send(Message::Text(json!({"type":"done","response": response}).to_string()))
            .await;
    }
}
