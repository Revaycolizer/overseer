//! WebSocket streaming endpoint for external UIs (e.g. the Tauri/React client).
//!
//! `GET /api/ws/chat` upgrades to a WebSocket. The client sends
//! `{"conversation_id": <i64>, "message": "<text>"}`; the server persists the
//! user message, runs it through the engine, and streams events:
//!   - `{"type":"token","text":...}`      incremental output
//!   - `{"type":"approval","id":...,"tool":...,"summary":...}` a governed action
//!     is waiting for a human — the client replies `{"approve":id}` / `{"reject":id}`
//!   - `{"type":"done","response":...}`   final answer
//!
//! Optional auth: if `OVERSEER_API_TOKEN` is set, the first frame must be
//! `{"token":"<value>"}` or the socket is closed.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use axum::Router;
use futures::sink::SinkExt;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use super::ServerState;

type Tx = SplitSink<WebSocket, Message>;
type Rx = SplitStream<WebSocket>;

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/api/ws/chat", get(ws_handler))
}

#[derive(Deserialize)]
struct ChatReq {
    conversation_id: i64,
    message: String,
    /// When true, speak the assistant's reply aloud (voice conversation).
    #[serde(default)]
    speak: bool,
}

#[derive(Deserialize)]
struct TokenFrame {
    token: String,
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<ServerState>>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<ServerState>) {
    let (mut tx, mut rx) = socket.split();

    // Optional token gate.
    if let Ok(expected) = std::env::var("OVERSEER_API_TOKEN") {
        if !expected.is_empty() {
            let ok = match rx.next().await {
                Some(Ok(Message::Text(t))) => serde_json::from_str::<TokenFrame>(&t)
                    .map(|f| f.token == expected)
                    .unwrap_or(false),
                _ => false,
            };
            if !ok {
                let _ = send(&mut tx, json!({"type":"error","error":"unauthorized"})).await;
                return;
            }
        }
    }

    while let Some(Ok(msg)) = rx.next().await {
        let text = match msg {
            Message::Text(t) => t,
            Message::Close(_) => break,
            _ => continue,
        };
        // Control frames handled outside a run.
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) {
            if v.get("approve").is_some() || v.get("reject").is_some() {
                continue;
            }
            // Voice dictation: record one utterance on the daemon and transcribe it.
            if v.get("voice").and_then(|x| x.as_bool()) == Some(true) {
                // Don't record while talking back, or the mic captures our own TTS.
                let mut waited = 0;
                while crate::engine::voice::tts::is_tts_speaking() && waited < 60 {
                    tokio::time::sleep(Duration::from_millis(150)).await;
                    waited += 1;
                }
                let raw = crate::engine::voice::wake_word::get_vad_threshold();
                let threshold = if raw <= 0.0 { 0.02 } else { raw };
                let frame = match crate::engine::voice::local_stt::record_and_transcribe(threshold).await {
                    Ok(t) if !t.trim().is_empty() => json!({"type":"voice_result","text": t.trim()}),
                    Ok(_) => json!({"type":"voice_error","error":"Didn't catch any speech. Check your mic, and install a local STT tool: pip install openai-whisper (or vosk)."}),
                    Err(e) => json!({"type":"voice_error","error": format!("Voice capture failed: {e}")}),
                };
                let _ = send(&mut tx, frame).await;
                continue;
            }
        }
        let Ok(req) = serde_json::from_str::<ChatReq>(&text) else {
            let _ = send(&mut tx, json!({"type":"error","error":"bad request"})).await;
            continue;
        };

        if run_turn(&mut tx, &mut rx, &state, req).await.is_err() {
            return; // client disconnected
        }
    }
}

/// Drive one chat turn: run the engine while forwarding tokens, surfacing
/// pending approvals, and accepting approve/reject from the client — all
/// concurrently so an approval-gated run never hangs the socket.
async fn run_turn(tx: &mut Tx, rx: &mut Rx, state: &Arc<ServerState>, req: ChatReq) -> Result<(), ()> {
    let db = state.db.clone();
    let agent = state.agent_system.clone();
    let conv = req.conversation_id;
    let message = req.message.clone();

    let _ = crate::backend::add_message(&db, conv, "user", &message, false).await;

    let (tok_tx, mut tok_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let db_task = db.clone();
    let mut handle = tokio::spawn(async move {
        let _ = &agent; // agent_system kept for future routing; loop uses the engine directly
        if let Some(cmd) = crate::engine::governed_commands::parse_governed(&message) {
            crate::engine::governed_commands::run_governed(&db_task, conv, cmd).await
        } else {
            // Plain messages go through the function-calling agent loop so the
            // assistant actually *does* things (search, open, run) instead of
            // narrating intent. Chit-chat with no tool call just returns text.
            crate::engine::agent_loop::run_agent(&db_task, conv, &message, Some(tok_tx)).await
        }
    });

    let mut seen: HashSet<String> = HashSet::new();
    let mut poll = tokio::time::interval(Duration::from_millis(400));
    let mut tok_done = false;

    loop {
        tokio::select! {
            biased;

            res = &mut handle => {
                let response = res.unwrap_or_default();
                let _ = crate::backend::add_message(&db, conv, "assistant", &response, false).await;
                if req.speak {
                    crate::engine::voice::tts::speak_locally(&response);
                }
                send(tx, json!({"type":"done","response": response})).await?;
                return Ok(());
            }

            tok = tok_rx.recv(), if !tok_done => {
                match tok {
                    Some(t) => send(tx, json!({"type":"token","text": t})).await?,
                    None => tok_done = true,
                }
            }

            incoming = rx.next() => {
                match incoming {
                    Some(Ok(Message::Text(t))) => {
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&t) {
                            if let Some(id) = v.get("approve").and_then(|x| x.as_str()) {
                                crate::engine::kernel::approve(&db, id, "user");
                                seen.remove(id);
                            } else if let Some(id) = v.get("reject").and_then(|x| x.as_str()) {
                                crate::engine::kernel::reject(&db, id, "user");
                                seen.remove(id);
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => return Err(()),
                    _ => {}
                }
            }

            _ = poll.tick() => {
                let pending = crate::engine::kernel::list_pending_approvals(&db, None).await;
                for a in pending {
                    if seen.insert(a.id.clone()) {
                        send(tx, json!({
                            "type": "approval",
                            "id": a.id,
                            "tool": a.tool_name,
                            "kind": a.action_kind,
                            "summary": a.summary.unwrap_or_default(),
                            "reason": a.reason.unwrap_or_default(),
                        })).await?;
                    }
                }
            }
        }
    }
}

async fn send(tx: &mut Tx, value: serde_json::Value) -> Result<(), ()> {
    tx.send(Message::Text(value.to_string())).await.map_err(|_| ())
}
