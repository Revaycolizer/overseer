use serde_json::json;
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::object_tool_input;
use crate::engine::tools::utils::{clean_text_content, truncate_for_tool};

pub async fn native_http_request(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let method = input
        .get("method")
        .and_then(|value| value.as_str())
        .unwrap_or("GET")
        .to_uppercase();
    let url = input
        .get("url")
        .and_then(|value| value.as_str())
        .or_else(|| {
            input
                .get("input")
                .and_then(|value| value.as_str())
                .filter(|value| value.starts_with("http://") || value.starts_with("https://"))
        })
        .ok_or_else(|| DynamicToolError::ExecutionFailed {
            tool: "http_request".to_string(),
            details: "Missing URL for HTTP request.".to_string(),
        })?;

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Overseer/0.1.3 (+local-first mission control)")
        .build()
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "http_request".to_string(),
            details: err.to_string(),
        })?;

    let method_parsed = reqwest::Method::from_bytes(method.as_bytes()).map_err(|err| {
        DynamicToolError::ExecutionFailed {
            tool: "http_request".to_string(),
            details: format!("Unsupported HTTP method '{}': {}", method, err),
        }
    })?;

    let mut request = client.request(method_parsed, url);
    if let Some(headers) = input.get("headers").and_then(|value| value.as_object()) {
        for (key, value) in headers {
            if let Some(as_str) = value.as_str() {
                request = request.header(key, as_str);
            }
        }
    }

    if let Some(body) = input.get("body") {
        if body.is_string() {
            request = request.body(body.as_str().unwrap_or_default().to_string());
        } else {
            request = request.body(body.to_string());
        }
    } else if let Some(json_body) = input.get("json") {
        request = request.json(json_body);
    }

    let response = request
        .send()
        .await
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "http_request".to_string(),
            details: err.to_string(),
        })?;

    let status = response.status();
    let final_url = response.url().to_string();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    let body = response
        .text()
        .await
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "http_request".to_string(),
            details: err.to_string(),
        })?;

    let body_preview = truncate_for_tool(clean_text_content(&body), 6000);

    Ok(json!({
        "status": status.as_u16(),
        "ok": status.is_success(),
        "url": final_url,
        "content_type": content_type,
        "body": body_preview,
    })
    .to_string())
}
