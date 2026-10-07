use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::stringify_tool_input;
use crate::engine::tools::utils::{clean_text_content, truncate_for_tool};
use regex::Regex;
use serde_json::json;

pub async fn native_web_links(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let mut url = stringify_tool_input(args);
    if url.starts_with("//") {
        url = format!("https:{}", url);
    }

    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "web_links".to_string(),
            details: "web_links requires a full http:// or https:// URL.".to_string(),
        });
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 Overseer/0.1.3")
        .build()
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "web_links".to_string(),
            details: err.to_string(),
        })?;

    let response =
        client
            .get(&url)
            .send()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "web_links".to_string(),
                details: err.to_string(),
            })?;

    let final_url = response.url().to_string();
    let body = response
        .text()
        .await
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "web_links".to_string(),
            details: err.to_string(),
        })?;

    let link_re =
        Regex::new(r#"(?is)<a[^>]+href=["'](?P<href>[^"']+)["'][^>]*>(?P<label>.*?)</a>"#).unwrap();
    let tag_re = Regex::new(r"(?is)<[^>]+>").unwrap();
    let base_url = reqwest::Url::parse(&final_url).ok();

    let mut seen = std::collections::HashSet::new();
    let mut links = Vec::new();
    for cap in link_re.captures_iter(&body) {
        if links.len() >= 40 {
            break;
        }

        let href_raw = cap.name("href").map(|m| m.as_str().trim()).unwrap_or("");
        if href_raw.is_empty()
            || href_raw.starts_with('#')
            || href_raw.starts_with("javascript:")
            || href_raw.starts_with("mailto:")
        {
            continue;
        }

        let href = if let Some(base) = &base_url {
            base.join(href_raw)
                .map(|u| u.to_string())
                .unwrap_or_else(|_| href_raw.to_string())
        } else {
            href_raw.to_string()
        };

        if !seen.insert(href.clone()) {
            continue;
        }

        let label_html = cap.name("label").map(|m| m.as_str()).unwrap_or("");
        let label = truncate_for_tool(
            clean_text_content(&tag_re.replace_all(label_html, " ")),
            160,
        );
        links.push(json!({
            "label": label,
            "url": href,
        }));
    }

    Ok(json!({
        "action": "web_links",
        "url": final_url,
        "links": links,
    })
    .to_string())
}
