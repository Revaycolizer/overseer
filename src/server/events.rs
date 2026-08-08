//! Server-push events over WebSocket (separate from chat, so it never touches the
//! chat flow). Currently: wake signals from the hands-free wake service — the
//! floating widget subscribes and summons itself on `{"type":"wake"}`.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use std::sync::Arc;

use super::ServerState;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/api/ws/events", get(ws_handler))
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<ServerState>>) -> Response {
    ws.on_upgrade(move |socket| handle(socket, state))
}

async fn handle(socket: WebSocket, state: Arc<ServerState>) {
    let (mut tx, mut rx) = socket.split();
    let mut wake = state.wake_tx.subscribe();
    loop {
        tokio::select! {
            signal = wake.recv() => {
                match signal {
                    Ok(()) => {
                        if tx.send(Message::Text("{\"type\":\"wake\"}".into())).await.is_err() {
                            break;
                        }
                    }
                    // Lagged just means we missed some — keep going.
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(_) => break,
                }
            }
            incoming = rx.next() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }
}
