use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::stringify_tool_input;
use crate::engine::tools::web_search::results::{parse_duckduckgo_results, SearchResult};

pub async fn native_stealth_search(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let query = stringify_tool_input(args);
    if query.trim().is_empty() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "stealth_search".to_string(),
            details: "stealth_search requires a search query.".to_string(),
        });
    }

    let client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(30))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "stealth_search".to_string(),
            details: err.to_string(),
        })?;

    let url = reqwest::Url::parse_with_params(
        "https://html.duckduckgo.com/html/",
        &[("q", query.as_str())],
    )
    .map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "stealth_search".to_string(),
        details: err.to_string(),
    })?;

    let mut last_err = None;
    let mut html = String::new();

    for attempt in 1..=3 {
        match client.get(url.clone()).send().await {
            Ok(resp) => {
                match resp.error_for_status() {
                    Ok(resp_ok) => {
                        if let Ok(text) = resp_ok.text().await {
                            html = text;
                            last_err = None;
                            break;
                        }
                    },
                    Err(e) => {
                        last_err = Some(format!("HTTP Error: {}", e));
                    }
                }
            },
            Err(e) => {
                last_err = Some(format!("Network Error: {}", e));
            }
        }

        if attempt < 3 {
            println!("⚠️ Search attempt {} failed, retrying in 1s...", attempt);
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    if let Some(err_details) = last_err {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "stealth_search".to_string(),
            details: format!("Failed after 3 attempts: {}", err_details),
        });
    }

    let mut results = parse_duckduckgo_results(&html);
    if results.is_empty() {
        return Ok(format!("No search results found for '{}'.", query.trim()));
    }

    // --- RE-RANKING ---
    let keywords: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .filter(|w| w.len() > 2)
        .map(|w| w.to_string())
        .collect();

    results.sort_by_cached_key(|r| {
        let title_lower = r.title.to_lowercase();
        let mut score = 0i32;
        for kw in &keywords {
            if title_lower.contains(kw) {
                score -= 10;
            }
        }
        score
    });

    let top_results: Vec<SearchResult> = results.into_iter().take(8).collect();

    Ok(format!(
        "---SEARCH_RESULTS---\n{}",
        serde_json::to_string(&top_results).unwrap_or_else(|_| "[]".to_string())
    ))
}
