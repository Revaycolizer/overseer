use serde_json::json;
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::stringify_tool_input;
use crate::engine::tools::utils::{clean_text_content, extract_html_title, truncate_for_tool};

pub async fn native_web_extract(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let mut url = stringify_tool_input(args);
    if url.starts_with("//") {
        url = format!("https:{}", url);
    }

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "web_extract".to_string(),
            details: "web_extract requires a full http:// or https:// URL.".to_string(),
        });
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 Overseer/0.1.3")
        .build()
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "web_extract".to_string(),
            details: err.to_string(),
        })?;

    let response =
        client
            .get(&url)
            .send()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "web_extract".to_string(),
                details: err.to_string(),
            })?;

    let final_url = response.url().to_string();
    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "web_extract".to_string(),
            details: err.to_string(),
        })?;

    let title = extract_html_title(&body).unwrap_or_else(|| "Untitled page".to_string());
    let content = truncate_for_tool(clean_text_content(&body), 6000);

    Ok(json!({
        "action": "web_extract",
        "url": final_url,
        "status": status,
        "title": title,
        "content": content,
    })
    .to_string())
}
