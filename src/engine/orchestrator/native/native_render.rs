use crate::engine::orchestrator::core::response::{MessagePart, StructuredResponse, UnknownPart};
use serde_json::Value;

pub fn render_native_os_tool_response(raw: &str) -> String {
    let value: Value = serde_json::from_str(raw).unwrap_or_else(|_| serde_json::json!({}));
    let action = value.get("action").and_then(|v| v.as_str()).unwrap_or("");
    let mut structured = StructuredResponse::new();

    match action {
        "screenshot" => {
            let width = value
                .get("width")
                .and_then(|v| v.as_i64())
                .unwrap_or_default();
            let height = value
                .get("height")
                .and_then(|v| v.as_i64())
                .unwrap_or_default();
            let path = value
                .get("path")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .trim();

            let msg = if width > 0 && height > 0 {
                format!("Captured a desktop screenshot at {}x{}.", width, height)
            } else {
                "Captured a desktop screenshot.".to_string()
            };
            structured.messages.push(MessagePart {
                message: msg,
                role: "assistant".to_string(),
            });
            if !path.is_empty() {
                structured.attachments.push(path.to_string());
            }
        }
        "launch" => {
            let app = value
                .get("app")
                .or_else(|| value.get("target"))
                .and_then(|v| v.as_str())
                .unwrap_or("requested app")
                .trim();
            structured.messages.push(MessagePart {
                message: format!("Opened {}.", app),
                role: "assistant".to_string(),
            });
        }
        "list_processes" => {
            let total = value
                .get("total_processes")
                .and_then(|v| v.as_i64())
                .unwrap_or_default();
            let msg = format!("Current processes captured. Total: {}.", total);
            structured.messages.push(MessagePart {
                message: msg,
                role: "assistant".to_string(),
            });
            structured.unknown.push(UnknownPart {
                data: value.clone(),
                kind: "text".to_string(),
            });
        }
        _ => {
            let summary = value
                .get("summary")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_else(|| raw.to_string());
            structured.messages.push(MessagePart {
                message: summary,
                role: "assistant".to_string(),
            });
        }
    }

    serde_json::to_string(&structured).unwrap_or_else(|_| raw.to_string())
}
