use futures::StreamExt;
use rig::agent::{MultiTurnStreamItem, StreamingResult};
use rig::streaming::StreamedAssistantContent;

/// Consume a `StreamingResult` from a rig-core 0.30 streaming
/// completion into a single `String` (the assembled final response).
///
/// Useful for paths that don't need incremental updates (e.g.
/// `generate_title`, summarisation, the fallback path of a streaming
/// call that errored). Mirrors the contract of `agent.prompt(...)`/
/// `agent.chat(...)` (which return a `String`) so call-sites can be
/// swapped in one line.
///
/// If `token_sender` is `Some`, each `StreamAssistantItem::Text` chunk
/// is pushed through the channel so the caller (e.g. the Dioxus chat)
/// can update its UI as the LLM streams. If `token_sender` is `None`,
/// the intermediate chunks are ignored (only the terminal
/// `FinalResponse` is captured).
pub async fn collect_stream_with_sender<R>(
    mut stream: StreamingResult<R>,
    token_sender: Option<&tokio::sync::mpsc::UnboundedSender<String>>,
) -> Result<String, String> {
    let mut final_response: Option<String> = None;
    while let Some(item) = stream.next().await {
        match item.map_err(|e| e.to_string())? {
            MultiTurnStreamItem::FinalResponse(r) => {
                final_response = Some(r.response().to_string());
            }
            MultiTurnStreamItem::StreamAssistantItem(
                StreamedAssistantContent::Text(text),
            ) => {
                if let Some(s) = token_sender {
                    let _ = s.send(text.text);
                }
            }
            _ => {}
        }
    }
    final_response.ok_or_else(|| "stream ended without FinalResponse".to_string())
}

/// Convenience: collect a stream with no token sink. Equivalent to
/// `collect_stream_with_sender(stream, None)`.
pub async fn collect_stream<R>(stream: StreamingResult<R>) -> Result<String, String> {
    collect_stream_with_sender(stream, None).await
}