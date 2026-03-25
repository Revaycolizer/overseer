use std::path::PathBuf;

#[cfg(windows)]
use enigo::{
    Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings,
};
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::process::Command;

use crate::engine::utils::discovery::{build_python_command, get_tool_executable_path, ToolInfo};

pub mod charting;

#[derive(Debug, Clone)]
pub enum DynamicToolError {
    MissingExecutable { tool: String },
    MissingPython { details: String },
    InvalidChartRequest(String),
    UnsupportedLanguage(String),
    NativeNotImplemented(String),
    ExecutionFailed { tool: String, details: String },
    TimedOut { tool: String, timeout_ms: u64 },
}

impl std::fmt::Display for DynamicToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingExecutable { tool } => write!(
                f,
                "No executable path could be resolved for tool '{}'.",
                tool
            ),
            Self::MissingPython { details } => write!(f, "Python runtime unavailable: {}", details),
            Self::InvalidChartRequest(details) => {
                write!(f, "Invalid chart request format: {}", details)
            }
            Self::UnsupportedLanguage(language) => {
                write!(f, "Unsupported tool language: {}", language)
            }
            Self::NativeNotImplemented(tool) => write!(
                f,
                "Native tool '{}' is not implemented in this build.",
                tool
            ),
            Self::ExecutionFailed { tool, details } => {
                write!(f, "Tool '{}' failed: {}", tool, details)
            }
            Self::TimedOut { tool, timeout_ms } => {
                write!(f, "Tool '{}' timed out after {} ms.", tool, timeout_ms)
            }
        }
    }
}

impl std::error::Error for DynamicToolError {}

async fn call_dynamic_tool_logic(
    info: &ToolInfo,
    args: serde_json::Value,
) -> Result<String, DynamicToolError> {
    if let Some(result) = call_native_builtin_tool(info, &args).await? {
        return Ok(result);
    }

    match info.language.as_str() {
        "python" => {
            let executable_path = get_tool_executable_path(info).ok_or_else(|| {
                DynamicToolError::MissingExecutable {
                    tool: info.name.clone(),
                }
            })?;

            let parts = build_python_command()
                .map_err(|details| DynamicToolError::MissingPython { details })?;
            let mut cmd = Command::new(&parts[0]);
            if parts.len() > 1 {
                cmd.args(&parts[1..]);
            }
            cmd.arg(&executable_path);
            cmd.kill_on_drop(true);
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                cmd.creation_flags(CREATE_NO_WINDOW);
            }

            let input_str = if let Some(input) = args.get("input") {
                if input.is_string() {
                    input.as_str().unwrap_or("").to_string()
                } else {
                    input.to_string()
                }
            } else if let Some(inner) = args.get("args") {
                inner.to_string()
            } else if args.is_object() {
                let map = args.as_object().unwrap();
                if map.len() == 1 {
                    let val = map.values().next().unwrap();
                    if val.is_string() {
                        val.as_str().unwrap().to_string()
                    } else {
                        val.to_string()
                    }
                } else if let Some(url) = map.get("url") {
                    if url.is_string() {
                        url.as_str().unwrap().to_string()
                    } else {
                        url.to_string()
                    }
                } else if let Some(query) = map.get("query") {
                    if query.is_string() {
                        query.as_str().unwrap().to_string()
                    } else {
                        query.to_string()
                    }
                } else {
                    args.to_string()
                }
            } else {
                args.to_string()
            };

            cmd.arg(input_str);

            let output = cmd
                .output()
                .await
                .map_err(|e| DynamicToolError::ExecutionFailed {
                    tool: info.name.clone(),
                    details: e.to_string(),
                })?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                Ok(stdout)
            } else {
                Err(DynamicToolError::ExecutionFailed {
                    tool: info.name.clone(),
                    details: format!("{}\n{}", stdout, stderr).trim().to_string(),
                })
            }
        }
        "rust" => match info.name.as_str() {
            "os_tool" => native_os_tool(&args).await,
            "vision_analysis" => native_vision_analysis(&args).await,
            "charting" => {
                let input_str = if let Some(input) = args.get("input") {
                    if input.is_string() {
                        input.as_str().unwrap_or("").to_string()
                    } else {
                        input.to_string()
                    }
                } else {
                    args.to_string()
                }
                .trim()
                .to_string();

                println!("📊 CHARTING INPUT: {}", input_str);

                let req: charting::ChartRequest = serde_json::from_str(&input_str)
                    .or_else(|_| {
                        let val: serde_json::Value = serde_json::from_str(&input_str)?;
                        serde_json::from_value(val)
                    })
                    .map_err(|e| {
                        DynamicToolError::InvalidChartRequest(format!(
                            "{}. Provide {{\"chart_type\":\"bar|line|pie\",\"title\":\"...\",\"labels\":[...],\"data\":[...]}}",
                            e
                        ))
                    })?;

                charting::generate_chart_svg(req).map_err(|details| {
                    DynamicToolError::ExecutionFailed {
                        tool: info.name.clone(),
                        details,
                    }
                })
            }
            _ => Err(DynamicToolError::NativeNotImplemented(info.name.clone())),
        },
        _ => Err(DynamicToolError::UnsupportedLanguage(info.language.clone())),
    }
}

async fn call_native_builtin_tool(
    info: &ToolInfo,
    args: &serde_json::Value,
) -> Result<Option<String>, DynamicToolError> {
    let output = match info.name.as_str() {
        "http_request" => Some(native_http_request(args).await?),
        "stealth_browse" => Some(native_stealth_browse(args).await?),
        "stealth_search" => Some(native_stealth_search(args).await?),
        "app_launcher" => Some(native_app_launcher(args).await?),
        _ => None,
    };

    Ok(output)
}

fn stringify_tool_input(args: &serde_json::Value) -> String {
    if let Some(input) = args.get("input") {
        if let Some(text) = input.as_str() {
            return text.to_string();
        }
        return input.to_string();
    }

    if let Some(query) = args.get("query").and_then(|value| value.as_str()) {
        return query.to_string();
    }

    if let Some(url) = args.get("url").and_then(|value| value.as_str()) {
        return url.to_string();
    }

    if let Some(name) = args.get("app").and_then(|value| value.as_str()) {
        return name.to_string();
    }

    args.to_string()
}

fn object_tool_input(args: &serde_json::Value) -> serde_json::Value {
    if let Some(input) = args.get("input") {
        if input.is_object() {
            return input.clone();
        }
        if let Some(raw) = input.as_str() {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(raw) {
                return parsed;
            }
            return json!({ "input": raw });
        }
        return json!({ "input": input.to_string() });
    }

    args.clone()
}

async fn native_http_request(args: &serde_json::Value) -> Result<String, DynamicToolError> {
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
        .user_agent("Overseer/0.1.2 (+local-first mission control)")
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

    let response = request.send().await.map_err(|err| DynamicToolError::ExecutionFailed {
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
    let body = response.text().await.map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "http_request".to_string(),
        details: err.to_string(),
    })?;

    let body_preview = truncate_for_tool(clean_text_content(&body), 6000);

    Ok(
        json!({
            "status": status.as_u16(),
            "ok": status.is_success(),
            "url": final_url,
            "content_type": content_type,
            "body": body_preview,
        })
        .to_string(),
    )
}

async fn native_stealth_browse(args: &serde_json::Value) -> Result<String, DynamicToolError> {
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
        .user_agent("Mozilla/5.0 Overseer/0.1.2")
        .build()
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "stealth_browse".to_string(),
            details: err.to_string(),
        })?;

    let response = client.get(&url).send().await.map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "stealth_browse".to_string(),
        details: err.to_string(),
    })?;

    let final_url = response.url().to_string();
    let status = response.status();
    let body = response.text().await.map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "stealth_browse".to_string(),
        details: err.to_string(),
    })?;

    let title = extract_html_title(&body).unwrap_or_else(|| "Untitled page".to_string());
    
    // Extract images before cleaning tags
    let mut image_markers = String::new();
    let img_re = Regex::new(r#"(?i)<img[^>]+src="(?P<src>https?://[^"]+)"[^>]*>"#).unwrap();
    let mut count = 0;
    for cap in img_re.captures_iter(&body) {
        if count >= 10 { break; }
        let src = &cap["src"];
        if !src.contains("favicon") && !src.contains("pixel") && !src.contains("logo") && !src.contains("icon") {
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

async fn native_stealth_search(args: &serde_json::Value) -> Result<String, DynamicToolError> {
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
        .user_agent("Mozilla/5.0 Overseer/0.1.2")
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

    let html = client
        .get(url)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "stealth_search".to_string(),
            details: err.to_string(),
        })?
        .text()
        .await
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "stealth_search".to_string(),
            details: err.to_string(),
        })?;

    let mut results = parse_duckduckgo_results(&html);
    if results.is_empty() {
        return Ok(format!(
            "No search results found for '{}'.",
            query.trim()
        ));
    }

    // --- RE-RANKING ---
    // Simple heuristic: count keyword matches in title
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
                score -= 10; // Lower is better for sort
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

async fn native_app_launcher(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let app = input
        .get("app")
        .and_then(|value| value.as_str())
        .or_else(|| input.get("input").and_then(|value| value.as_str()))
        .ok_or_else(|| DynamicToolError::ExecutionFailed {
            tool: "app_launcher".to_string(),
            details: "Missing app name to launch.".to_string(),
        })?;

    let mut command = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg("start").arg("").arg(app);
        cmd
    } else if cfg!(target_os = "macos") {
        let mut cmd = Command::new("open");
        cmd.arg("-a").arg(app);
        cmd
    } else {
        Command::new(app)
    };

    command.kill_on_drop(false);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command.spawn().map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "app_launcher".to_string(),
        details: err.to_string(),
    })?;

    Ok(format!("Launched '{}'.", app))
}

fn read_string_field(input: &serde_json::Value, keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        input
            .get(*key)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    })
}

fn read_i64_field(input: &serde_json::Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| {
        input
            .get(*key)
            .and_then(|value| value.as_i64().or_else(|| value.as_str()?.trim().parse().ok()))
    })
}

fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

fn infer_os_tool_action(input: &serde_json::Value) -> String {
    let explicit = read_string_field(input, &["action", "command", "mode"])
        .unwrap_or_else(|| stringify_tool_input(input));
    let lower = explicit.trim().to_lowercase();

    if lower.is_empty() {
        return "get_state".to_string();
    }

    if matches!(
        lower.as_str(),
        "get_state" | "state" | "observe" | "inspect" | "snapshot" | "status"
    ) || lower.contains("current state")
        || lower.contains("observe desktop")
        || lower.contains("screen state")
    {
        return "get_state".to_string();
    }

    if matches!(lower.as_str(), "list_windows" | "windows" | "list")
        || (lower.contains("list") && lower.contains("window"))
    {
        return "list_windows".to_string();
    }

    if matches!(
        lower.as_str(),
        "list_processes" | "processes" | "tasks" | "background_tasks" | "background processes"
    ) || ((lower.contains("background") || lower.contains("running"))
        && (lower.contains("task") || lower.contains("process")))
        || lower.contains("what's running")
        || lower.contains("whats running")
    {
        return "list_processes".to_string();
    }

    if matches!(lower.as_str(), "find_files" | "search_files" | "locate_file")
        || (lower.contains("find") && (lower.contains("file") || lower.contains("path")))
        || (lower.contains("locate") && lower.contains("file"))
        || (lower.contains("search") && lower.contains("file"))
    {
        return "find_files".to_string();
    }

    if matches!(lower.as_str(), "read_file" | "read" | "show_file")
        || ((lower.contains("read") || lower.contains("show"))
            && (lower.contains("file") || lower.contains(".txt") || lower.contains(".md") || lower.contains(".json")))
    {
        return "read_file".to_string();
    }

    if matches!(lower.as_str(), "open_file" | "open document" | "open path")
        || (lower.contains("open") && (lower.contains("file") || lower.contains("document") || lower.contains("path")))
    {
        return "open_file".to_string();
    }

    if matches!(lower.as_str(), "browser_state" | "browser" | "tab_state")
        || (lower.contains("browser") && (lower.contains("state") || lower.contains("open") || lower.contains("which")))
        || lower.contains("current tab")
        || lower.contains("current page")
    {
        return "browser_state".to_string();
    }

    if matches!(lower.as_str(), "focus" | "activate" | "activate_window")
        || lower.starts_with("focus ")
        || lower.starts_with("activate ")
    {
        return "focus".to_string();
    }

    if matches!(
        lower.as_str(),
        "screenshot" | "capture" | "capture_screen" | "screen_capture"
    ) || lower.contains("take screenshot")
        || lower.contains("capture screen")
    {
        return "screenshot".to_string();
    }

    if matches!(lower.as_str(), "double_click" | "double click")
        || lower.starts_with("double click")
    {
        return "double_click".to_string();
    }

    if matches!(lower.as_str(), "right_click" | "right click")
        || lower.starts_with("right click")
    {
        return "right_click".to_string();
    }

    if matches!(lower.as_str(), "click" | "left_click" | "tap")
        || lower.starts_with("click ")
        || lower.starts_with("tap ")
    {
        return "click".to_string();
    }

    if matches!(lower.as_str(), "move_mouse" | "move cursor" | "move pointer")
        || lower.starts_with("move mouse")
        || lower.starts_with("move cursor")
    {
        return "move_mouse".to_string();
    }

    if matches!(lower.as_str(), "type" | "type_text" | "enter_text" | "input_text")
        || lower.starts_with("type ")
        || lower.starts_with("enter text ")
        || lower.starts_with("input text ")
    {
        return "type_text".to_string();
    }

    if matches!(lower.as_str(), "press_key" | "keypress" | "key_press")
        || lower.starts_with("press ")
    {
        return "press_key".to_string();
    }

    if matches!(lower.as_str(), "hotkey" | "shortcut" | "key_combo" | "combo")
        || lower.starts_with("hotkey ")
        || lower.starts_with("shortcut ")
        || lower.starts_with("combo ")
    {
        return "hotkey".to_string();
    }

    if matches!(lower.as_str(), "scroll" | "scroll_down" | "scroll_up")
        || lower.starts_with("scroll ")
    {
        return "scroll".to_string();
    }

    if matches!(lower.as_str(), "launch" | "open_app" | "start_app")
        || lower.starts_with("launch ")
        || lower.starts_with("open app ")
        || lower.starts_with("start app ")
    {
        return "launch".to_string();
    }

    if matches!(lower.as_str(), "open_url" | "browse" | "navigate")
        || lower.starts_with("open ")
        || lower.starts_with("browse ")
        || lower.starts_with("navigate ")
        || lower.starts_with("http://")
        || lower.starts_with("https://")
    {
        return "open_url".to_string();
    }

    if matches!(lower.as_str(), "wait" | "sleep" | "pause")
        || lower.starts_with("wait ")
        || lower.starts_with("sleep ")
        || lower.starts_with("pause ")
    {
        return "wait".to_string();
    }

    "get_state".to_string()
}

fn extract_os_tool_target(input: &serde_json::Value, action: &str) -> Option<String> {
    if let Some(value) = read_string_field(
        input,
        &["target", "title", "window", "app", "application", "url", "query", "filename", "name"],
    ) {
        return Some(value);
    }

    let raw = read_string_field(input, &["input"])?;
    let lower = raw.to_lowercase();

    match action {
        "focus" => {
            if let Some(rest) = raw.strip_prefix("focus ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("activate ") {
                return Some(rest.trim().to_string());
            }
        }
        "launch" => {
            if let Some(rest) = raw.strip_prefix("launch ") {
                return Some(rest.trim().to_string());
            }
            if lower.starts_with("open app ") {
                return Some(raw["open app ".len()..].trim().to_string());
            }
            if lower.starts_with("start app ") {
                return Some(raw["start app ".len()..].trim().to_string());
            }
        }
        "open_url" => {
            if raw.starts_with("http://") || raw.starts_with("https://") {
                return Some(raw);
            }
            if let Some(rest) = raw.strip_prefix("open ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("browse ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("navigate ") {
                return Some(rest.trim().to_string());
            }
        }
        "find_files" => {
            if let Some(rest) = raw.strip_prefix("find ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("search for ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("locate ") {
                return Some(rest.trim().to_string());
            }
        }
        "read_file" | "open_file" => {
            if let Some(rest) = raw.strip_prefix("read ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("open ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("show ") {
                return Some(rest.trim().to_string());
            }
        }
        _ => {}
    }

    None
}

fn extract_text_value(input: &serde_json::Value) -> Option<String> {
    if let Some(value) = read_string_field(input, &["text", "value", "content"]) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let raw = read_string_field(input, &["input"])?;
    let lower = raw.to_lowercase();
    for prefix in ["type ", "enter text ", "input text "] {
        if lower.starts_with(prefix) {
            let extracted = raw[prefix.len()..].trim();
            if !extracted.is_empty() {
                return Some(extracted.to_string());
            }
        }
    }

    None
}

fn extract_coordinates(input: &serde_json::Value) -> Option<(i32, i32)> {
    let x = read_i64_field(input, &["x", "left"]).map(|value| value as i32);
    let y = read_i64_field(input, &["y", "top"]).map(|value| value as i32);
    match (x, y) {
        (Some(x), Some(y)) => return Some((x, y)),
        _ => {}
    }

    if let Some(coords) = input.get("coords").and_then(|value| value.as_array()) {
        if coords.len() >= 2 {
            let x = coords.first().and_then(|value| value.as_i64())? as i32;
            let y = coords.get(1).and_then(|value| value.as_i64())? as i32;
            return Some((x, y));
        }
    }

    None
}

fn extract_key_tokens(input: &serde_json::Value) -> Vec<String> {
    if let Some(keys) = input.get("keys").and_then(|value| value.as_array()) {
        let tokens: Vec<String> = keys
            .iter()
            .filter_map(|value| value.as_str())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if !tokens.is_empty() {
            return tokens;
        }
    }

    if let Some(key) = read_string_field(input, &["key", "shortcut", "combo"]) {
        return key
            .split(|c| matches!(c, '+' | ',' | ' '))
            .map(str::trim)
            .filter(|token| !token.is_empty())
            .map(ToString::to_string)
            .collect();
    }

    if let Some(raw) = read_string_field(input, &["input"]) {
        let lower = raw.to_lowercase();
        let prefixes = [
            "press ",
            "hotkey ",
            "shortcut ",
            "key combo ",
            "combo ",
        ];
        for prefix in prefixes {
            if lower.starts_with(prefix) {
                return raw[prefix.len()..]
                    .split(|c| matches!(c, '+' | ',' | ' '))
                    .map(str::trim)
                    .filter(|token| !token.is_empty())
                    .map(ToString::to_string)
                    .collect();
            }
        }
    }

    Vec::new()
}

#[cfg(windows)]
fn key_from_token(token: &str) -> Option<Key> {
    let lower = token.trim().to_lowercase();
    Some(match lower.as_str() {
        "ctrl" | "control" => Key::Control,
        "alt" | "option" => Key::Alt,
        "shift" => Key::Shift,
        "meta" | "win" | "windows" | "super" | "cmd" | "command" => Key::Meta,
        "enter" | "return" => Key::Return,
        "tab" => Key::Tab,
        "esc" | "escape" => Key::Escape,
        "backspace" => Key::Backspace,
        "delete" | "del" => Key::Delete,
        "space" => Key::Space,
        "up" => Key::UpArrow,
        "down" => Key::DownArrow,
        "left" => Key::LeftArrow,
        "right" => Key::RightArrow,
        "pageup" | "page_up" => Key::PageUp,
        "pagedown" | "page_down" => Key::PageDown,
        "home" => Key::Home,
        "end" => Key::End,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        _ => {
            let mut chars = lower.chars();
            let first = chars.next()?;
            if chars.next().is_none() {
                Key::Unicode(first)
            } else {
                return None;
            }
        }
    })
}

#[cfg(windows)]
fn with_enigo() -> Result<Enigo, DynamicToolError> {
    Enigo::new(&Settings::default()).map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "os_tool".to_string(),
        details: format!("Failed to initialize native desktop input: {}", err),
    })
}

#[cfg(windows)]
fn runtime_artifacts_dir() -> Result<PathBuf, DynamicToolError> {
    let dir = crate::engine::utils::discovery::runtime_dir().join("artifacts");
    std::fs::create_dir_all(&dir).map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "vision_analysis".to_string(),
        details: format!("Failed to prepare runtime artifacts directory: {}", err),
    })?;
    Ok(dir)
}

#[cfg(windows)]
async fn capture_windows_screenshot_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let timestamp = chrono::Utc::now().format("%Y%m%d-%H%M%S-%3f");
    let output_path = runtime_artifacts_dir()?.join(format!("desktop-{}.png", timestamp));
    let output_path_str = output_path.to_string_lossy().replace('\'', "''");
    let script = format!(
        r#"
Add-Type -AssemblyName System.Windows.Forms
Add-Type -AssemblyName System.Drawing
$bounds = [System.Windows.Forms.SystemInformation]::VirtualScreen
$bitmap = New-Object System.Drawing.Bitmap $bounds.Width, $bounds.Height
$graphics = [System.Drawing.Graphics]::FromImage($bitmap)
$graphics.CopyFromScreen($bounds.Left, $bounds.Top, 0, 0, $bitmap.Size)
$bitmap.Save('{path}', [System.Drawing.Imaging.ImageFormat]::Png)
$graphics.Dispose()
$bitmap.Dispose()
[pscustomobject]@{{
    action = 'screenshot'
    path = '{path}'
    width = $bounds.Width
    height = $bounds.Height
    left = $bounds.Left
    top = $bounds.Top
}} | ConvertTo-Json -Compress
"#,
        path = output_path_str
    );

    run_powershell_hidden(&script, tool_name).await
}

async fn run_powershell_hidden(script: &str, tool_name: &str) -> Result<String, DynamicToolError> {
    let mut command = Command::new("powershell");
    command
        .arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-Command")
        .arg(script);
    command.kill_on_drop(true);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command
        .output()
        .await
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: err.to_string(),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: if stdout.is_empty() {
                stderr
            } else if stderr.is_empty() {
                stdout
            } else {
                format!("{}\n{}", stdout, stderr)
            },
        })
    }
}

#[cfg(windows)]
async fn windows_desktop_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let script = r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
using System.Text;
public static class OverseerWinApi {
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();

    [DllImport("user32.dll", CharSet = CharSet.Unicode)]
    public static extern int GetWindowText(IntPtr hWnd, StringBuilder text, int count);

    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
}
"@
$hwnd = [OverseerWinApi]::GetForegroundWindow()
$activeTitle = ""
$activePid = 0
if ($hwnd -ne [IntPtr]::Zero) {
    $sb = New-Object System.Text.StringBuilder 1024
    [void][OverseerWinApi]::GetWindowText($hwnd, $sb, $sb.Capacity)
    $activeTitle = $sb.ToString()
    [void][OverseerWinApi]::GetWindowThreadProcessId($hwnd, [ref]$activePid)
}
$windows = Get-Process |
    Where-Object { $_.MainWindowTitle -and $_.MainWindowTitle.Trim().Length -gt 0 } |
    Select-Object -First 12 ProcessName, Id, MainWindowTitle
[pscustomobject]@{
    active_window_title = $activeTitle
    active_pid = $activePid
    visible_windows = $windows
} | ConvertTo-Json -Depth 4 -Compress
"#;

    run_powershell_hidden(script, tool_name).await
}

#[cfg(windows)]
async fn windows_process_list_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let script = r#"
$all = Get-Process
$top = $all |
    Sort-Object -Property CPU -Descending |
    Select-Object -First 20 ProcessName, Id, CPU, WS, MainWindowTitle
$background = $all |
    Where-Object { -not $_.MainWindowTitle -or $_.MainWindowTitle.Trim().Length -eq 0 } |
    Sort-Object -Property CPU -Descending |
    Select-Object -First 20 ProcessName, Id, CPU, WS
[pscustomobject]@{
    action = 'list_processes'
    total_processes = @($all).Count
    background_count = @($background).Count
    top_processes = $top
    background_processes = $background
} | ConvertTo-Json -Depth 5 -Compress
"#;

    run_powershell_hidden(script, tool_name).await
}

#[cfg(windows)]
async fn windows_find_files_json(
    tool_name: &str,
    query: &str,
) -> Result<String, DynamicToolError> {
    let escaped_query = escape_powershell_single_quoted(query);
    let script = format!(
        r#"
$query = '{query}'
$home = [Environment]::GetFolderPath('UserProfile')
$roots = @(
    (Join-Path $home 'Desktop'),
    (Join-Path $home 'Documents'),
    (Join-Path $home 'Downloads'),
    (Join-Path $home 'OneDrive')
) | Where-Object {{ $_ -and (Test-Path $_) }} | Select-Object -Unique

$tokens = $query.ToLower().Split(@(' ', '?', '.', ',', ':', ';', '\', '/'), [System.StringSplitOptions]::RemoveEmptyEntries) |
    Where-Object {{ $_.Length -ge 3 -and $_ -notin @('find','path','file','files','pc','computer','my','the','in') }}
if (-not $tokens -or $tokens.Count -eq 0) {{
    $tokens = @($query.ToLower())
}}

$results = New-Object System.Collections.Generic.List[object]
foreach ($root in $roots) {{
    try {{
        Get-ChildItem -Path $root -Recurse -File -ErrorAction SilentlyContinue |
            Where-Object {{
                $name = $_.Name.ToLower()
                $full = $_.FullName.ToLower()
                foreach ($token in $tokens) {{
                    if ($name -like "*$token*" -or $full -like "*$token*") {{ return $true }}
                }}
                return $false
            }} |
            Select-Object -First 12 |
            ForEach-Object {{
                $results.Add([pscustomobject]@{{
                    name = $_.Name
                    path = $_.FullName
                    directory = $_.DirectoryName
                    extension = $_.Extension
                }})
            }}
    }} catch {{}}
}}

[pscustomobject]@{{
    action = 'find_files'
    query = $query
    roots = $roots
    results = $results
}} | ConvertTo-Json -Depth 5 -Compress
"#,
        query = escaped_query
    );

    run_powershell_hidden(&script, tool_name).await
}

#[cfg(windows)]
async fn windows_read_file_json(
    tool_name: &str,
    path: &str,
) -> Result<String, DynamicToolError> {
    let escaped_path = escape_powershell_single_quoted(path);
    let script = format!(
        r#"
$path = '{path}'
if (-not (Test-Path $path)) {{
    throw "File not found: $path"
}}
$item = Get-Item $path
$content = $null
if ($item.Extension -in @('.txt','.md','.json','.rs','.toml','.yaml','.yml','.log','.csv')) {{
    $content = Get-Content -Path $path -Raw -ErrorAction SilentlyContinue
    if ($content -and $content.Length -gt 4000) {{
        $content = $content.Substring(0, 4000)
    }}
}}
[pscustomobject]@{{
    action = 'read_file'
    path = $item.FullName
    name = $item.Name
    extension = $item.Extension
    size = $item.Length
    preview = $content
}} | ConvertTo-Json -Depth 4 -Compress
"#,
        path = escaped_path
    );

    run_powershell_hidden(&script, tool_name).await
}

#[cfg(windows)]
async fn windows_open_file_json(
    tool_name: &str,
    path: &str,
) -> Result<String, DynamicToolError> {
    let escaped_path = escape_powershell_single_quoted(path);
    let script = format!(
        r#"
$path = '{path}'
if (-not (Test-Path $path)) {{
    throw "File not found: $path"
}}
Start-Process -FilePath $path
[pscustomobject]@{{
    action = 'open_file'
    path = (Resolve-Path $path).Path
    ok = $true
}} | ConvertTo-Json -Compress
"#,
        path = escaped_path
    );

    run_powershell_hidden(&script, tool_name).await
}

#[cfg(windows)]
async fn windows_browser_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let script = r#"
$browserNames = @('chrome','msedge','firefox','brave','opera')
$windows = Get-Process |
    Where-Object { $browserNames -contains $_.ProcessName.ToLower() -and $_.MainWindowTitle -and $_.MainWindowTitle.Trim().Length -gt 0 } |
    Select-Object -First 12 ProcessName, Id, MainWindowTitle
[pscustomobject]@{
    action = 'browser_state'
    browser_windows = $windows
    browser_count = @($windows).Count
} | ConvertTo-Json -Depth 4 -Compress
"#;

    run_powershell_hidden(script, tool_name).await
}

#[cfg(not(windows))]
async fn capture_windows_screenshot_json(tool_name: &str) -> Result<String, DynamicToolError> {
    Err(DynamicToolError::NativeNotImplemented(tool_name.to_string()))
}

#[cfg(not(windows))]
async fn windows_process_list_json(tool_name: &str) -> Result<String, DynamicToolError> {
    run_command_simple("ps -eo pid,comm,%cpu,rss --sort=-%cpu", tool_name).await
}

#[cfg(not(windows))]
async fn windows_find_files_json(
    tool_name: &str,
    _query: &str,
) -> Result<String, DynamicToolError> {
    Err(DynamicToolError::ExecutionFailed {
        tool: tool_name.to_string(),
        details: "Native file search is currently implemented for Windows only.".to_string(),
    })
}

#[cfg(not(windows))]
async fn windows_read_file_json(
    tool_name: &str,
    _path: &str,
) -> Result<String, DynamicToolError> {
    Err(DynamicToolError::ExecutionFailed {
        tool: tool_name.to_string(),
        details: "Native file read is currently implemented for Windows only.".to_string(),
    })
}

#[cfg(not(windows))]
async fn windows_open_file_json(
    tool_name: &str,
    _path: &str,
) -> Result<String, DynamicToolError> {
    Err(DynamicToolError::ExecutionFailed {
        tool: tool_name.to_string(),
        details: "Native file open is currently implemented for Windows only.".to_string(),
    })
}

#[cfg(not(windows))]
async fn windows_browser_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    Err(DynamicToolError::ExecutionFailed {
        tool: tool_name.to_string(),
        details: "Native browser state inspection is currently implemented for Windows only.".to_string(),
    })
}

#[cfg(not(windows))]
async fn windows_desktop_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    // Get active window title using xprop
    let active_window_id = run_command_simple("xprop -root _NET_ACTIVE_WINDOW", tool_name).await?;
    let mut active_title = String::new();
    if let Some(id) = active_window_id.split_whitespace().last() {
        let title_out = run_command_simple(&format!("xprop -id {} _NET_WM_NAME", id), tool_name).await?;
        if let Some(pos) = title_out.find(" = \"") {
            active_title = title_out[pos + 4..].trim_end_matches('"').to_string();
        } else {
            // Fallback to WM_NAME if _NET_WM_NAME is missing
            let title_out_fallback = run_command_simple(&format!("xprop -id {} WM_NAME", id), tool_name).await?;
            if let Some(pos) = title_out_fallback.find(" = \"") {
                active_title = title_out_fallback[pos + 4..].trim_end_matches('"').to_string();
            }
        }
    }

    // List windows using wmctrl
    let windows_raw = run_command_simple("wmctrl -l", tool_name).await.unwrap_or_default();
    let visible_windows: Vec<serde_json::Value> = windows_raw.lines().take(12).map(|line| {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let name = if parts.len() >= 4 {
            parts[3..].join(" ")
        } else {
            "Unknown Window".to_string()
        };
        json!({
            "ProcessName": "unknown",
            "Id": parts.get(0).copied().unwrap_or("0"),
            "MainWindowTitle": name
        })
    }).collect();

    Ok(json!({
        "active_window_title": active_title,
        "active_pid": 0,
        "visible_windows": visible_windows
    }).to_string())
}

#[cfg(not(windows))]
async fn run_command_simple(cmd: &str, tool_name: &str) -> Result<String, DynamicToolError> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() { return Ok(String::new()); }
    
    let output = Command::new(parts[0])
        .args(&parts[1..])
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: e.to_string(),
        })?;
    
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

async fn native_os_tool(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let action = infer_os_tool_action(&input);

    match action.as_str() {
        "get_state" | "list_windows" => windows_desktop_state_json("os_tool").await,
        "browser_state" => windows_browser_state_json("os_tool").await,
        "list_processes" => windows_process_list_json("os_tool").await,
        "find_files" => {
            let query = extract_os_tool_target(&input, "find_files").unwrap_or_else(|| stringify_tool_input(&input));
            windows_find_files_json("os_tool", &query).await
        }
        "read_file" => {
            let path = extract_os_tool_target(&input, "read_file").ok_or_else(|| DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: "Read file action requires a target path.".to_string(),
            })?;
            windows_read_file_json("os_tool", &path).await
        }
        "open_file" => {
            let path = extract_os_tool_target(&input, "open_file").ok_or_else(|| DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: "Open file action requires a target path.".to_string(),
            })?;
            windows_open_file_json("os_tool", &path).await
        }
        "screenshot" => capture_windows_screenshot_json("os_tool").await,
        "focus" => {
            #[cfg(windows)]
            {
                let target = extract_os_tool_target(&input, "focus").ok_or_else(|| {
                    DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: "Focus action requires a target window title.".to_string(),
                    }
                })?;
                let escaped = escape_powershell_single_quoted(&target);
                let script = format!(
                    "$wshell = New-Object -ComObject WScript.Shell; \
                     $ok = $wshell.AppActivate('{target}'); \
                     [pscustomobject]@{{ action='focus'; target='{target}'; activated=$ok }} | ConvertTo-Json -Compress",
                    target = escaped
                );
                run_powershell_hidden(&script, "os_tool").await
            }
            #[cfg(not(windows))]
            {
                Err(DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Window focus is currently implemented for Windows only.".to_string(),
                })
            }
        }
        "launch" => {
            let app = extract_os_tool_target(&input, "launch").ok_or_else(|| {
                DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Launch action requires an app or command.".to_string(),
                }
            })?;
            let escaped = escape_powershell_single_quoted(&app);
            let script = format!(
                "Start-Process '{app}'; [pscustomobject]@{{ action='launch'; app='{app}'; ok=$true }} | ConvertTo-Json -Compress",
                app = escaped
            );
            run_powershell_hidden(&script, "os_tool").await
        }
        "open_url" => {
            let url = extract_os_tool_target(&input, "open_url").ok_or_else(|| {
                DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Open URL action requires a target URL.".to_string(),
                }
            })?;
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Open URL only supports full http:// or https:// links.".to_string(),
                });
            }
            let escaped = escape_powershell_single_quoted(&url);
            let script = format!(
                "Start-Process '{url}'; [pscustomobject]@{{ action='open_url'; url='{url}'; ok=$true }} | ConvertTo-Json -Compress",
                url = escaped
            );
            run_powershell_hidden(&script, "os_tool").await
        }
        "click" | "double_click" | "right_click" => {
            #[cfg(windows)]
            {
                let mut enigo = with_enigo()?;
                let (x, y) = extract_coordinates(&input).ok_or_else(|| {
                    DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!(
                            "{} action requires `x` and `y` coordinates.",
                            action
                        ),
                    }
                })?;
                enigo
                    .move_mouse(x, y, Coordinate::Abs)
                    .map_err(|err| DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("Failed to move mouse: {}", err),
                    })?;
                let button = if action == "right_click" {
                    Button::Right
                } else {
                    Button::Left
                };
                enigo
                    .button(button, Direction::Click)
                    .map_err(|err| DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("Failed to click mouse: {}", err),
                    })?;
                if action == "double_click" {
                    enigo
                        .button(button, Direction::Click)
                        .map_err(|err| DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("Failed to complete double click: {}", err),
                        })?;
                }
                Ok(json!({
                    "action": action,
                    "x": x,
                    "y": y,
                    "button": if action == "right_click" { "right" } else { "left" },
                    "ok": true
                })
                .to_string())
            }
            #[cfg(not(windows))]
            {
                Err(DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Mouse click actions are currently implemented for Windows only.".to_string(),
                })
            }
        }
        "move_mouse" => {
            #[cfg(windows)]
            {
                let mut enigo = with_enigo()?;
                let (x, y) = extract_coordinates(&input).ok_or_else(|| {
                    DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: "Move mouse requires `x` and `y` coordinates.".to_string(),
                    }
                })?;
                enigo
                    .move_mouse(x, y, Coordinate::Abs)
                    .map_err(|err| DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("Failed to move mouse: {}", err),
                    })?;
                Ok(json!({
                    "action": "move_mouse",
                    "x": x,
                    "y": y,
                    "ok": true
                })
                .to_string())
            }
            #[cfg(not(windows))]
            {
                Err(DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Mouse movement is currently implemented for Windows only.".to_string(),
                })
            }
        }
        "type_text" => {
            #[cfg(windows)]
            {
                let mut enigo = with_enigo()?;
                let text = extract_text_value(&input).ok_or_else(|| {
                    DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: "Type text requires a `text` value.".to_string(),
                    }
                })?;
                enigo
                    .text(&text)
                    .map_err(|err| DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("Failed to type text: {}", err),
                    })?;
                Ok(json!({
                    "action": "type_text",
                    "typed_chars": text.chars().count(),
                    "ok": true
                })
                .to_string())
            }
            #[cfg(not(windows))]
            {
                Err(DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Typing is currently implemented for Windows only.".to_string(),
                })
            }
        }
        "press_key" => {
            #[cfg(windows)]
            {
                let mut enigo = with_enigo()?;
                let key = extract_key_tokens(&input).into_iter().next().ok_or_else(|| {
                    DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: "Press key requires a `key` value.".to_string(),
                    }
                })?;
                let resolved = key_from_token(&key).ok_or_else(|| DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: format!("Unsupported key token '{}'.", key),
                })?;
                enigo
                    .key(resolved, Direction::Click)
                    .map_err(|err| DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("Failed to press key: {}", err),
                    })?;
                Ok(json!({
                    "action": "press_key",
                    "key": key,
                    "ok": true
                })
                .to_string())
            }
            #[cfg(not(windows))]
            {
                Err(DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Key presses are currently implemented for Windows only.".to_string(),
                })
            }
        }
        "hotkey" => {
            #[cfg(windows)]
            {
                let mut enigo = with_enigo()?;
                let keys = extract_key_tokens(&input);
                if keys.len() < 2 {
                    return Err(DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: "Hotkey requires at least two key tokens, like `Ctrl+L`.".to_string(),
                    });
                }
                let resolved: Vec<Key> = keys
                    .iter()
                    .map(|token| {
                        key_from_token(token).ok_or_else(|| DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("Unsupported key token '{}'.", token),
                        })
                    })
                    .collect::<Result<_, _>>()?;

                for key in &resolved[..resolved.len() - 1] {
                    enigo
                        .key(*key, Direction::Press)
                        .map_err(|err| DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("Failed to press hotkey modifier: {}", err),
                        })?;
                }
                let final_key = *resolved.last().expect("checked len");
                enigo
                    .key(final_key, Direction::Click)
                    .map_err(|err| DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("Failed to trigger hotkey: {}", err),
                    })?;
                for key in resolved[..resolved.len() - 1].iter().rev() {
                    enigo
                        .key(*key, Direction::Release)
                        .map_err(|err| DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("Failed to release hotkey modifier: {}", err),
                        })?;
                }
                Ok(json!({
                    "action": "hotkey",
                    "keys": keys,
                    "ok": true
                })
                .to_string())
            }
            #[cfg(not(windows))]
            {
                Err(DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Hotkeys are currently implemented for Windows only.".to_string(),
                })
            }
        }
        "scroll" => {
            #[cfg(windows)]
            {
                let mut enigo = with_enigo()?;
                let amount = read_i64_field(&input, &["amount", "delta", "lines"])
                    .unwrap_or_else(|| {
                        let direction = read_string_field(&input, &["direction"])
                            .unwrap_or_else(|| stringify_tool_input(&input))
                            .to_lowercase();
                        if direction.contains("up") {
                            -3
                        } else {
                            3
                        }
                    }) as i32;
                let axis = match read_string_field(&input, &["axis"])
                    .unwrap_or_else(|| "vertical".to_string())
                    .to_lowercase()
                    .as_str()
                {
                    "horizontal" | "x" => Axis::Horizontal,
                    _ => Axis::Vertical,
                };
                enigo
                    .scroll(amount, axis)
                    .map_err(|err| DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("Failed to scroll: {}", err),
                    })?;
                Ok(json!({
                    "action": "scroll",
                    "amount": amount,
                    "axis": if matches!(axis, Axis::Horizontal) { "horizontal" } else { "vertical" },
                    "ok": true
                })
                .to_string())
            }
            #[cfg(not(windows))]
            {
                Err(DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Scrolling is currently implemented for Windows only.".to_string(),
                })
            }
        }
        "wait" => {
            let duration_ms =
                read_i64_field(&input, &["milliseconds", "ms", "duration_ms", "duration"])
                    .unwrap_or(1000)
                    .clamp(0, 30_000) as u64;
            tokio::time::sleep(std::time::Duration::from_millis(duration_ms)).await;
            Ok(json!({
                "action": "wait",
                "duration_ms": duration_ms,
                "ok": true
            })
            .to_string())
        }
        other => Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!(
                "Unsupported os_tool action '{}'. Supported actions: get_state, list_windows, browser_state, list_processes, find_files, read_file, open_file, screenshot, focus, click, double_click, right_click, move_mouse, type_text, press_key, hotkey, scroll, launch, open_url, wait.",
                other
            ),
        }),
    }
}

async fn native_vision_analysis(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let desktop_state = windows_desktop_state_json("vision_analysis").await?;
    let screenshot = capture_windows_screenshot_json("vision_analysis").await?;

    let source = read_string_field(&input, &["image", "path", "url", "input"])
        .unwrap_or_else(|| "live desktop session".to_string());

    Ok(json!({
        "mode": "desktop_snapshot",
        "source": source,
        "note": "Native vision analysis reports desktop state, visible windows, and a screenshot artifact. OCR and full UI semantic understanding are still limited.",
        "state": serde_json::from_str::<serde_json::Value>(&desktop_state).unwrap_or_else(|_| json!({ "raw": desktop_state })),
        "screenshot": serde_json::from_str::<serde_json::Value>(&screenshot).unwrap_or_else(|_| json!({ "raw": screenshot })),
    })
    .to_string())
}

#[derive(Debug, Serialize, Deserialize)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    image_url: Option<String>,
}

fn parse_duckduckgo_results(html: &str) -> Vec<SearchResult> {
    let item_re = Regex::new(
        r#"(?s)<div[^>]*class="[^"]*result[^"]*"[^>]*>.*?<a[^>]*class="[^"]*result__a[^"]*"[^>]*href="(?P<url>[^"]+)"[^>]*>(?P<title>.*?)</a>.*?(?:<a[^>]*class="[^"]*result__snippet[^"]*"[^>]*>|<div[^>]*class="[^"]*result__snippet[^"]*"[^>]*>)(?P<snippet>.*?)</(?:a|div)>"#,
    )
    .expect("valid search result regex");

    item_re
        .captures_iter(html)
        .map(|capture| {
            let mut url = decode_html_entities(capture.name("url").map(|value| value.as_str()).unwrap_or_default());
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
                title: clean_text_content(capture.name("title").map(|value| value.as_str()).unwrap_or_default()),
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

fn extract_html_title(html: &str) -> Option<String> {
    let title_re = Regex::new(r"(?is)<title[^>]*>(.*?)</title>").ok()?;
    title_re
        .captures(html)
        .and_then(|capture| capture.get(1))
        .map(|value| clean_text_content(value.as_str()))
        .filter(|value| !value.is_empty())
}

fn clean_text_content(raw: &str) -> String {
    let script_re = Regex::new(r"(?is)<script[^>]*>.*?</script>").expect("valid script regex");
    let style_re = Regex::new(r"(?is)<style[^>]*>.*?</style>").expect("valid style regex");
    let comment_re = Regex::new(r"(?is)<!--.*?-->").expect("valid comment regex");
    let tag_re = Regex::new(r"(?is)<[^>]+>").expect("valid tag regex");
    let whitespace_re = Regex::new(r"\s+").expect("valid whitespace regex");

    let without_scripts = script_re.replace_all(raw, " ");
    let without_styles = style_re.replace_all(&without_scripts, " ");
    let without_comments = comment_re.replace_all(&without_styles, " ");
    let without_tags = tag_re.replace_all(&without_comments, " ");
    let decoded = decode_html_entities(&without_tags);

    whitespace_re.replace_all(decoded.trim(), " ").to_string()
}

fn decode_html_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
}

fn truncate_for_tool(value: String, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value;
    }

    value.chars().take(max_chars).collect::<String>() + "..."
}

pub async fn call_dynamic_tool(
    info: &ToolInfo,
    args: serde_json::Value,
) -> Result<String, DynamicToolError> {
    call_dynamic_tool_logic(info, args).await
}

pub struct DynamicToolRouter {
    pub tools: Vec<ToolInfo>,
}

impl DynamicToolRouter {
    pub fn new(tools: Vec<ToolInfo>) -> Self {
        Self { tools }
    }
}

impl Tool for DynamicToolRouter {
    const NAME: &'static str = "execute_tool";
    type Args = serde_json::Value;
    type Output = String;
    type Error = DynamicToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let tool_list = self
            .tools
            .iter()
            .map(|t| format!("{}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        let enum_values = self.tools.iter().map(|t| json!(t.name)).collect::<Vec<_>>();

        ToolDefinition {
            name: "execute_tool".to_string(),
            description: format!(
                "Execute specialized tools for this supervisor. Available sub-tools:\n{}",
                tool_list
            ),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tool_name": {
                        "type": "string",
                        "enum": enum_values,
                        "description": "The name of the sub-tool to execute"
                    },
                    "input": {
                        "type": "string",
                        "description": "Main input for the selected tool (e.g. search query or URL)"
                    }
                },
                "required": ["tool_name", "input"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let tool_name = args
            .get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DynamicToolError::ExecutionFailed {
                tool: "execute_tool".to_string(),
                details: "Missing tool_name".to_string(),
            })?;

        let tool_info = self
            .tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| DynamicToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                details: "Tool not found in this router".to_string(),
            })?;

        call_dynamic_tool_logic(tool_info, args).await
    }
}

#[derive(Clone)]
pub struct StructuredPerceptionTool {
    pub db: crate::db::Database,
}

impl StructuredPerceptionTool {
    pub fn new(db: crate::db::Database) -> Self {
        Self { db }
    }
}
