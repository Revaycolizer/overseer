use futures::StreamExt;
use rig::agent::{MultiTurnStreamItem, StreamingResult};

/// Consume a `StreamingResult` from a rig-core 0.30 streaming
/// completion into a single `String` (the assembled final response).
///
/// Useful for paths that don't need incremental updates (e.g.
/// `generate_title`, summarisation, the fallback path of a streaming
/// call that errored). Mirrors the contract of `agent.prompt(...)`/
/// `agent.chat(...)` (which return a `String`) so call-sites can be
/// swapped in one line.
///
/// The intermediate `StreamAssistantItem` chunks are ignored — only
/// the terminal `FinalResponse` is captured. For incremental
/// per-token updates, callers should iterate the stream directly
/// instead of going through this helper.
pub async fn collect_stream<R>(mut stream: StreamingResult<R>) -> Result<String, String> {
    let mut final_response: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item.map_err(|e| e.to_string())? {
            MultiTurnStreamItem::FinalResponse(r) => {
                final_response = Some(r.response().to_string());
            }
            _ => {}
        }
    }
    final_response.ok_or_else(|| "stream ended without FinalResponse".to_string())
}