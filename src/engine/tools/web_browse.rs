use regex::Regex;
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::stringify_tool_input;
use crate::engine::tools::utils::{clean_text_content, extract_html_title, truncate_for_tool};

pub async fn native_stealth_browse(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let mut url = stringify_tool_input(args);
    if url.starts_with("//") {
        url = format!("https:{}", url);
    }

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "stealth_browse".to_string(),
            details: "stealth_browse requires a full http:// or https:// URL.".to_string(),
        });
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 Overseer/0.1.3")
        .build()
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "stealth_browse".to_string(),
            details: err.to_string(),
        })?;

    let response =
        client
            .get(&url)
            .send()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "stealth_browse".to_string(),
                details: err.to_string(),
            })?;

    let final_url = response.url().to_string();
    let status = response.status();
    let body = response
        .text()
        .await
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "stealth_browse".to_string(),
            details: err.to_string(),
        })?;

    let title = extract_html_title(&body).unwrap_or_else(|| "Untitled page".to_string());

    // Extract images before cleaning tags
    let mut image_markers = String::new();
    let img_re = Regex::new(r#"(?i)<img[^>]+src="(?P<src>https?://[^"]+)"[^>]*>"#).unwrap();
    let mut count = 0;
    for cap in img_re.captures_iter(&body) {
        if count >= 10 {
            break;
        }
        let src = &cap["src"];
        if !src.contains("favicon")
            && !src.contains("pixel")
            && !src.contains("logo")
            && !src.contains("icon")
        {
            image_markers.push_str(&format!("[IMAGE: {}]\n", src));
            count += 1;
        }
    }

    let text = truncate_for_tool(clean_text_content(&body), 8000);

    Ok(format!(
        "URL: {}\nStatus: {}\nTitle: {}\n\n--- VISUAL ASSETS ---\n{}\n\n--- PAGE CONTENT ---\n{}",
        final_url, status, title, image_markers, text
    ))
}
