use serde::{Deserialize, Serialize};
use regex::Regex;
use crate::engine::tools::utils::{clean_text_content, decode_html_entities};

#[derive(Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_url: Option<String>,
}

pub fn parse_duckduckgo_results(html: &str) -> Vec<SearchResult> {
    let item_re = Regex::new(
        r#"(?s)<div[^>]*class="[^"]*result[^"]*"[^>]*>.*?<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="(?P<url>[^"]+)"[^>]*>(?P<title>.*?)</a>.*?(?:<a[^>]*class="[^"]*result__snippet[^"]*"[^>]*>|<div[^>]*class="[^"]*result__snippet[^"]*"[^>]*>)(?P<snippet>.*?)</(?:a|div)>"#,
    )
    .expect("valid search result regex");

    item_re
        .captures_iter(html)
        .map(|capture| {
            let mut url = decode_html_entities(
                capture
                    .name("url")
                    .map(|value| value.as_str())
                    .unwrap_or_default(),
            );
            if url.starts_with("//") {
                url = format!("https:{}", url);
            }

            // Extract the actual URL from DuckDuckGo redirect if present
            if url.contains("duckduckgo.com/l/?uddg=") {
                if let Some(pos) = url.find("uddg=") {
                    let encoded = &url[pos + 5..];
                    let end = encoded.find('&').unwrap_or(encoded.len());
                    let part = &encoded[..end];
                    if let Ok(decoded) = urlencoding::decode(part) {
                        url = decoded.to_string();
                    }
                }
            }

            SearchResult {
                title: clean_text_content(
                    capture
                        .name("title")
                        .map(|value| value.as_str())
                        .unwrap_or_default(),
                ),
                url,
                snippet: clean_text_content(
                    capture
                        .name("snippet")
                        .map(|value| value.as_str())
                        .unwrap_or_default(),
                ),
                image_url: None,
            }
        })
        .filter(|result| !result.title.is_empty() && !result.url.is_empty())
        .collect()
}
