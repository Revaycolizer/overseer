use std::path::PathBuf;

use enigo::{Axis, Button, Coordinate, Direction, Enigo, Key, Keyboard, Mouse, Settings};
use regex::Regex;
use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::process::Stdio;
use tokio::process::Command;

use crate::engine::utils::discovery::{build_python_command, get_tool_executable_path, ToolInfo};

pub mod charting;
pub mod document_gen;
pub mod document_generation;

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

                // Validate that we CAN render it, but return the structured JSON data as the result
                charting::generate_chart_svg(req.clone()).map_err(|details| {
                    DynamicToolError::ExecutionFailed {
                        tool: info.name.clone(),
                        details,
                    }
                })?;

                Ok(serde_json::to_string(&req).unwrap_or(input_str))
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
        "clipboard_read" => Some(native_clipboard_read().await?),
        "clipboard_write" => Some(native_clipboard_write(args).await?),
        "http_request" => Some(native_http_request(args).await?),
        "web_extract" => Some(native_web_extract(args).await?),
        "web_links" => Some(native_web_links(args).await?),
        "web_browse" => Some(native_stealth_browse(args).await?),
        "stealth_browse" => Some(native_stealth_browse(args).await?),
        "web_search" => Some(native_stealth_search(args).await?),
        "stealth_search" => Some(native_stealth_search(args).await?),
        "app_launcher" => Some(native_app_launcher(args).await?),
        "document_writer" => {
            println!("🛠️ DEBUG: Calling native_document_writer");
            match native_document_writer(args).await {
                Ok(r) => {
                    println!("🛠️ DEBUG: native_document_writer returned: {}", r);
                    Some(r)
                },
                Err(e) => {
                    println!("❌ DEBUG: native_document_writer failed: {}", e);
                    return Err(e);
                }
            }
        },
        "export_asset" => Some(native_export_asset(args).await?),
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

    command
        .spawn()
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "app_launcher".to_string(),
            details: err.to_string(),
        })?;

    Ok(format!("Launched '{}'.", app))
}

async fn native_document_writer(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    document_generation::native_document_writer(args)
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: "document_writer".to_string(),
            details: e,
        })
}

pub async fn native_export_asset(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let asset_path = input
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DynamicToolError::ExecutionFailed {
            tool: "export_asset".to_string(),
            details: "Missing 'path' to export.".to_string(),
        })?;

    let raw_source = std::path::Path::new(asset_path);
    let source_path_buf = if raw_source.exists() {
        raw_source.to_path_buf()
    } else {
        let clean_path = asset_path.trim_start_matches('/');
        let rel_path = std::path::Path::new(clean_path);
        if rel_path.exists() {
            rel_path.to_path_buf()
        } else {
            let data_dir = crate::db::get_overseer_data_dir();
            let data_path = data_dir.join(clean_path);
            if data_path.exists() {
                data_path
            } else {
                return Err(DynamicToolError::ExecutionFailed {
                    tool: "export_asset".to_string(),
                    details: format!("Source asset not found: {}", asset_path),
                });
            }
        }
    };
    let source = &source_path_buf;

    let filename = source.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("exported_file");

    let download_dir = dirs::download_dir().unwrap_or_else(|| {
        #[cfg(not(windows))]
        { PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string())).join("Downloads") }
        #[cfg(windows)]
        { PathBuf::from(std::env::var("USERPROFILE").unwrap_or_else(|_| "C:\\".to_string())).join("Downloads") }
    });

    if !download_dir.exists() {
        let _ = std::fs::create_dir_all(&download_dir);
    }

    let destination = download_dir.join(filename);
    
    std::fs::copy(source, &destination).map_err(|e| DynamicToolError::ExecutionFailed {
        tool: "export_asset".to_string(),
        details: format!("Failed to copy to Downloads: {}", e),
    })?;

    Ok(json!({
        "status": "success",
        "message": format!("File exported to Downloads: {}", filename),
        "destination": destination.display().to_string()
    }).to_string())
}


async fn native_clipboard_read() -> Result<String, DynamicToolError> {
    #[cfg(windows)]
    {
        let output = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Get-Clipboard"])
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_read".to_string(),
                details: err.to_string(),
            })?;

        if !output.status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_read".to_string(),
                details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(json!({
            "action": "clipboard_read",
            "text": text,
        })
        .to_string());
    }

    #[cfg(target_os = "macos")]
    {
        let output = Command::new("pbpaste")
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_read".to_string(),
                details: err.to_string(),
            })?;

        if !output.status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_read".to_string(),
                details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }

        let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Ok(json!({
            "action": "clipboard_read",
            "text": text,
        })
        .to_string());
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let candidates = [
            ("wl-paste", vec!["-n"]),
            ("xclip", vec!["-o", "-selection", "clipboard"]),
            ("xsel", vec!["--clipboard", "--output"]),
        ];

        for (cmd, args) in candidates {
            let mut command = Command::new(cmd);
            command.args(args).kill_on_drop(true);
            if let Ok(output) = command.output().await {
                if output.status.success() {
                    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    return Ok(json!({
                        "action": "clipboard_read",
                        "text": text,
                    })
                    .to_string());
                }
            }
        }

        Err(DynamicToolError::ExecutionFailed {
            tool: "clipboard_read".to_string(),
            details: "No supported clipboard reader found (tried wl-paste, xclip, xsel)."
                .to_string(),
        })
    }
}

async fn native_clipboard_write(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let text = stringify_tool_input(args);
    if text.trim().is_empty() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: "clipboard_write".to_string(),
            details: "clipboard_write requires text input.".to_string(),
        });
    }

    #[cfg(windows)]
    {
        let escaped = text.replace('\'', "''");
        let output = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!("Set-Clipboard -Value '{}'", escaped),
            ])
            .kill_on_drop(true)
            .output()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: err.to_string(),
            })?;

        if !output.status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
            });
        }
    }

    #[cfg(target_os = "macos")]
    {
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: err.to_string(),
            })?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            stdin.write_all(text.as_bytes()).await.map_err(|err| {
                DynamicToolError::ExecutionFailed {
                    tool: "clipboard_write".to_string(),
                    details: err.to_string(),
                }
            })?;
        }

        let status = child
            .wait()
            .await
            .map_err(|err| DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: err.to_string(),
            })?;

        if !status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: "pbcopy failed.".to_string(),
            });
        }
    }

    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let candidates = [
            ("wl-copy", Vec::<&str>::new()),
            ("xclip", vec!["-selection", "clipboard"]),
            ("xsel", vec!["--clipboard", "--input"]),
        ];

        let mut succeeded = false;
        for (cmd, args) in candidates {
            let spawned = Command::new(cmd)
                .args(args)
                .stdin(Stdio::piped())
                .kill_on_drop(true)
                .spawn();

            let Ok(mut child) = spawned else {
                continue;
            };

            if let Some(mut stdin) = child.stdin.take() {
                use tokio::io::AsyncWriteExt;
                if stdin.write_all(text.as_bytes()).await.is_err() {
                    continue;
                }
            }

            if let Ok(status) = child.wait().await {
                if status.success() {
                    succeeded = true;
                    break;
                }
            }
        }

        if !succeeded {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "clipboard_write".to_string(),
                details: "No supported clipboard writer found (tried wl-copy, xclip, xsel)."
                    .to_string(),
            });
        }
    }

    Ok(json!({
        "action": "clipboard_write",
        "written": true,
        "characters": text.chars().count(),
    })
    .to_string())
}

async fn native_web_extract(args: &serde_json::Value) -> Result<String, DynamicToolError> {
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

async fn native_web_links(args: &serde_json::Value) -> Result<String, DynamicToolError> {
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
        input.get(*key).and_then(|value| {
            value
                .as_i64()
                .or_else(|| value.as_str()?.trim().parse().ok())
        })
    })
}

fn escape_powershell_single_quoted(value: &str) -> String {
    value.replace('\'', "''")
}

fn tokenize_file_query(query: &str) -> Vec<String> {
    let mut tokens: Vec<String> = query
        .to_lowercase()
        .split(|ch: char| matches!(ch, ' ' | '?' | '.' | ',' | ':' | ';' | '\\' | '/'))
        .map(str::trim)
        .filter(|token| {
            token.len() >= 3
                && !matches!(
                    *token,
                    "find" | "path" | "file" | "files" | "pc" | "computer" | "my" | "the" | "in"
                )
        })
        .map(|token| token.to_string())
        .collect();

    if tokens.is_empty() {
        let trimmed = query.trim().to_lowercase();
        if !trimmed.is_empty() {
            tokens.push(trimmed);
        }
    }

    tokens
}

fn score_file_match(
    path_lower: &str,
    name_lower: &str,
    query_lower: &str,
    tokens: &[String],
) -> i32 {
    let mut score = 0;

    if name_lower == query_lower {
        score += 200;
    }

    for token in tokens {
        if name_lower == token {
            score += 80;
        }
        if name_lower.contains(token) {
            score += 30;
        }
        if path_lower.contains(token) {
            score += 10;
        }
    }

    if path_lower.contains("\\overseer\\src\\") || path_lower.contains("/overseer/src/") {
        score += 40;
    }
    if path_lower.contains("\\desktop\\") || path_lower.contains("/desktop/") {
        score += 5;
    }

    score
}

async fn search_workspace_files_fast(query: &str) -> Option<Vec<serde_json::Value>> {
    let cwd = std::env::current_dir().ok()?;
    let query_lower = query.trim().to_lowercase();
    if query_lower.is_empty() {
        return None;
    }

    let tokens = tokenize_file_query(query);
    let mut command = Command::new("rg");
    command.arg("--files").arg(&cwd);
    command.kill_on_drop(true);

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let output = command.output().await.ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut matches: Vec<(i32, PathBuf)> = stdout
        .lines()
        .filter_map(|line| {
            let relative = line.trim();
            if relative.is_empty() {
                return None;
            }

            let absolute = cwd.join(relative);
            let path_lower = absolute.to_string_lossy().to_lowercase();
            let name_lower = absolute
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_lowercase();

            let score = score_file_match(&path_lower, &name_lower, &query_lower, &tokens);
            if score <= 0 {
                return None;
            }

            Some((score, absolute))
        })
        .collect();

    matches.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.to_string_lossy().cmp(&right.1.to_string_lossy()))
    });

    matches.dedup_by(|left, right| left.1 == right.1);

    let results = matches
        .into_iter()
        .take(20)
        .map(|(_, path)| {
            json!({
                "name": path.file_name().and_then(|value| value.to_str()).unwrap_or_default(),
                "path": path.to_string_lossy().to_string(),
                "directory": path.parent().map(|value| value.to_string_lossy().to_string()).unwrap_or_default(),
                "extension": path.extension().map(|value| format!(".{}", value.to_string_lossy())).unwrap_or_default(),
            })
        })
        .collect::<Vec<_>>();

    if results.is_empty() {
        None
    } else {
        Some(results)
    }
}

fn infer_os_tool_action(input: &serde_json::Value) -> String {
    let explicit = read_string_field(input, &["action", "command", "mode", "query"])
        .or_else(|| read_string_field(input, &["input"]))
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
        "inspect_accessibility"
            | "accessibility"
            | "accessibility_tree"
            | "ui_tree"
            | "focused_element"
            | "uia"
    ) || lower.contains("accessibility tree")
        || lower.contains("ui automation")
        || lower.contains("automation tree")
        || lower.contains("focused element")
        || lower.contains("focused control")
        || lower.contains("inspect accessibility")
        || lower.contains("inspect ui")
    {
        return "inspect_accessibility".to_string();
    }

    if matches!(
        lower.as_str(),
        "list_processes" | "processes" | "tasks" | "background_tasks" | "background processes"
    ) || ((lower.contains("background") || lower.contains("running"))
        && (lower.contains("task") || lower.contains("process")))
        || lower.contains("what's running")
        || lower.contains("whats running")
        || ((lower.contains("currently running")
            || lower.contains("what is running")
            || lower.contains("what is currently running"))
            && (lower.contains("computer")
                || lower.contains("pc")
                || lower.contains("machine")
                || lower.contains("system")))
    {
        return "list_processes".to_string();
    }

    if matches!(
        lower.as_str(),
        "find_files" | "search_files" | "locate_file"
    ) || (lower.contains("find") && (lower.contains("file") || lower.contains("path")))
        || (lower.contains("locate") && lower.contains("file"))
        || (lower.contains("search") && lower.contains("file"))
    {
        return "find_files".to_string();
    }

    if matches!(lower.as_str(), "read_file" | "read" | "show_file")
        || ((lower.contains("read") || lower.contains("show"))
            && (lower.contains("file")
                || lower.contains(".txt")
                || lower.contains(".md")
                || lower.contains(".json")))
    {
        return "read_file".to_string();
    }

    if matches!(lower.as_str(), "open_file" | "open document" | "open path")
        || (lower.contains("open")
            && (lower.contains("file")
                || lower.contains("document")
                || lower.contains("path")
                || lower.contains("downloads")
                || lower.contains("documents")
                || lower.contains("pictures")
                || lower.contains("videos")
                || lower.contains("music")
                || lower.contains("desktop")))
    {
        return "open_file".to_string();
    }

    if matches!(lower.as_str(), "browser_state" | "browser" | "tab_state")
        || (lower.contains("browser")
            && (lower.contains("state") || lower.contains("open") || lower.contains("which")))
        || lower.contains("current tab")
        || lower.contains("current page")
    {
        return "browser_state".to_string();
    }

    if matches!(lower.as_str(), "focus_element" | "focus control")
        || lower.starts_with("focus element ")
        || lower.starts_with("focus control ")
    {
        return "focus_element".to_string();
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
        || lower.contains("take a screenshot")
        || lower.contains("screeshot")
        || lower.contains("screen shot")
        || lower.contains("capture screen")
    {
        return "screenshot".to_string();
    }

    if matches!(lower.as_str(), "double_click" | "double click")
        || lower.starts_with("double click")
    {
        return "double_click".to_string();
    }

    if matches!(lower.as_str(), "right_click" | "right click") || lower.starts_with("right click") {
        return "right_click".to_string();
    }

    if matches!(
        lower.as_str(),
        "invoke" | "invoke_element" | "invoke control"
    ) || lower.starts_with("invoke ")
        || lower.starts_with("click element ")
        || lower.starts_with("press button ")
        || lower.starts_with("toggle ")
        || lower.starts_with("expand ")
        || lower.starts_with("collapse ")
    {
        return "invoke_element".to_string();
    }

    if matches!(lower.as_str(), "click" | "left_click" | "tap")
        || lower.starts_with("click ")
        || lower.starts_with("tap ")
    {
        return "click".to_string();
    }

    if matches!(
        lower.as_str(),
        "move_mouse" | "move cursor" | "move pointer"
    ) || lower.starts_with("move mouse")
        || lower.starts_with("move cursor")
    {
        return "move_mouse".to_string();
    }

    if matches!(
        lower.as_str(),
        "type" | "type_text" | "enter_text" | "input_text"
    ) || lower.starts_with("type ")
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

    if matches!(
        lower.as_str(),
        "hotkey" | "shortcut" | "key_combo" | "combo"
    ) || lower.starts_with("hotkey ")
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
        || lower.starts_with("open application ")
        || lower.starts_with("open program ")
        || lower.starts_with("start app ")
        || lower.starts_with("start application ")
        || lower.starts_with("start program ")
        || ((lower.contains("open ") || lower.contains("start ") || lower.contains("launch "))
            && (lower.contains("pc")
                || lower.contains("computer")
                || lower.contains("app")
                || lower.contains("application")
                || lower.contains("program")
                || lower.contains("calculator")
                || lower.contains("terminal")
                || lower.contains("console")
                || lower.contains("shell")
                || lower.contains("notepad")
                || lower.contains("discord")
                || lower.contains("chrome")
                || lower.contains("spotify")
                || lower.contains("vscode")
                || lower.contains("visual studio code")
                || lower.contains("explorer")))
    {
        return "launch".to_string();
    }

    if matches!(lower.as_str(), "open_url" | "browse" | "navigate")
        || lower.contains("open website")
        || lower.contains("open url")
        || lower.contains("open link")
        || lower.starts_with("open http://")
        || lower.starts_with("open https://")
        || lower.contains("visit ")
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
        &[
            "target",
            "title",
            "window",
            "app",
            "application",
            "url",
            "filename",
            "name",
            "path",
        ],
    ) {
        return Some(value);
    }

    let raw = read_string_field(input, &["query", "input"])?;
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
        "focus_element" => {
            if let Some(rest) = lower.strip_prefix("focus element ") {
                return Some(raw[raw.len() - rest.len()..].trim().to_string());
            }
            if let Some(rest) = lower.strip_prefix("focus control ") {
                return Some(raw[raw.len() - rest.len()..].trim().to_string());
            }
        }
        "open_file" | "launch" => {
            let mut res: Option<String> = None;
            for prefix in ["open ", "launch ", "start "] {
                if let Some(rest) = lower.strip_prefix(prefix) {
                    res = Some(raw[raw.len() - rest.len()..].to_string());
                    break;
                }
            }
            if let Some(target) = res {
                let mut clean = target.trim().to_string();
                let lower_clean = clean.to_lowercase();
                for suffix in [
                    " file",
                    " folder",
                    " app",
                    " application",
                    " directory",
                    " program",
                    " windows",
                ] {
                    if lower_clean.ends_with(suffix) {
                        clean = clean[..clean.len() - suffix.len()].trim().to_string();
                        break;
                    }
                }
                return Some(clean);
            }
        }
        "invoke_element" => {
            for prefix in ["invoke ", "click element ", "press button "] {
                if let Some(rest) = lower.strip_prefix(prefix) {
                    return Some(raw[raw.len() - rest.len()..].trim().to_string());
                }
            }
        }
        "launch" => {
            if let Some(rest) = raw.strip_prefix("launch ") {
                return Some(rest.trim().to_string());
            }
            if lower.starts_with("open app ") {
                return Some(raw["open app ".len()..].trim().to_string());
            }
            if lower.starts_with("open application ") {
                return Some(raw["open application ".len()..].trim().to_string());
            }
            if lower.starts_with("open program ") {
                return Some(raw["open program ".len()..].trim().to_string());
            }
            if lower.starts_with("start app ") {
                return Some(raw["start app ".len()..].trim().to_string());
            }
            if lower.starts_with("start application ") {
                return Some(raw["start application ".len()..].trim().to_string());
            }
            if lower.starts_with("start program ") {
                return Some(raw["start program ".len()..].trim().to_string());
            }
            if let Some(index) = lower.find("open ") {
                let rest = raw[index + "open ".len()..].trim();
                let cleaned = rest
                    .trim_start_matches("a ")
                    .trim_start_matches("an ")
                    .trim_start_matches("the ")
                    .trim();
                let lowered = cleaned.to_lowercase();
                let suffixes = [
                    " on my pc",
                    " on my computer",
                    " on pc",
                    " on computer",
                    " on this pc",
                    " on this computer",
                ];
                for suffix in suffixes {
                    if lowered.ends_with(suffix) {
                        let end = cleaned.len().saturating_sub(suffix.len());
                        return Some(cleaned[..end].trim().to_string());
                    }
                }
                return Some(cleaned.to_string());
            }
            if let Some(index) = lower.find("start ") {
                let rest = raw[index + "start ".len()..].trim();
                let cleaned = rest
                    .trim_start_matches("a ")
                    .trim_start_matches("an ")
                    .trim_start_matches("the ")
                    .trim();
                return Some(cleaned.to_string());
            }
        }
        "open_url" => {
            if raw.starts_with("http://") || raw.starts_with("https://") {
                return Some(raw);
            }
            if let Some(rest) = raw.strip_prefix("open ") {
                let trimmed = rest.trim();
                if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
                    return Some(trimmed.to_string());
                }
            }
            if let Some(rest) = raw.strip_prefix("open url ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("open website ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("open link ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("browse ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("navigate ") {
                return Some(rest.trim().to_string());
            }
            if let Some(rest) = raw.strip_prefix("visit ") {
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

fn infer_uia_control_type(text: &str) -> Option<&'static str> {
    let lower = text.to_lowercase();
    if lower.contains("checkbox")
        || lower.contains("check box")
        || lower.contains("toggle")
        || lower.contains("switch")
    {
        return Some("CheckBox");
    }
    if lower.contains("radio button") || lower.contains("radio") {
        return Some("RadioButton");
    }
    if lower.contains("button") {
        return Some("Button");
    }
    if lower.contains("textbox")
        || lower.contains("text box")
        || lower.contains("input")
        || lower.contains("field")
        || lower.contains("edit")
        || lower.contains("search box")
        || lower.contains("search field")
        || lower.contains("address bar")
        || lower.contains("address field")
        || lower.contains("command palette")
        || lower.contains("palette")
    {
        return Some("Edit");
    }
    if lower.contains("tree item")
        || lower.contains("sidebar section")
        || lower.contains("sidebar item")
        || lower.contains("section")
    {
        return Some("TreeItem");
    }
    if lower.contains("list item") || lower.contains("list row") {
        return Some("ListItem");
    }
    if lower.contains("tab") {
        return Some("TabItem");
    }
    if lower.contains("link") {
        return Some("Hyperlink");
    }
    if lower.contains("menu item") || lower.contains("menu") {
        return Some("MenuItem");
    }
    if lower.contains("combo box") || lower.contains("dropdown") || lower.contains("drop-down") {
        return Some("ComboBox");
    }
    None
}

fn clean_uia_selector_name(text: &str) -> String {
    let mut cleaned = text.trim().to_string();
    for prefix in ["the ", "a ", "an ", "this ", "that ", "my ", "current "] {
        if cleaned.to_lowercase().starts_with(prefix) {
            cleaned = cleaned[prefix.len()..].trim().to_string();
        }
    }

    for suffix in [
        " button",
        " field",
        " input",
        " textbox",
        " text box",
        " edit",
        " box",
        " checkbox",
        " check box",
        " toggle",
        " switch",
        " radio button",
        " radio",
        " tab",
        " link",
        " menu item",
        " menu",
        " dropdown",
        " drop-down",
        " combo box",
        " search box",
        " search field",
        " address bar",
        " address field",
        " command palette",
        " palette",
        " sidebar section",
        " sidebar item",
        " tree item",
        " list item",
        " section",
        " item",
        " control",
        " element",
    ] {
        if cleaned.to_lowercase().ends_with(suffix) {
            let end = cleaned.len().saturating_sub(suffix.len());
            cleaned = cleaned[..end].trim().to_string();
            break;
        }
    }

    cleaned
}

fn infer_uia_app_hint_details(raw: &str) -> Option<(&'static str, &'static [&'static str])> {
    let lower = raw.to_lowercase();
    let mappings: &[(&str, &[&str])] = &[
        ("discord", &["discord"]),
        ("chrome", &["google chrome", "chrome"]),
        ("msedge", &["microsoft edge", "edge", "msedge"]),
        (
            "visual studio code",
            &["visual studio code", "vs code", "vscode"],
        ),
        (
            "explorer",
            &["file explorer", "windows explorer", "explorer"],
        ),
        ("spotify", &["spotify"]),
    ];

    mappings.iter().find_map(|(canonical, aliases)| {
        if aliases.iter().any(|alias| lower.contains(alias)) {
            Some((*canonical, *aliases))
        } else {
            None
        }
    })
}

fn infer_uia_app_hint(input: &serde_json::Value) -> Option<String> {
    for field in ["app_hint", "app", "application"] {
        if let Some(value) = read_string_field(input, &[field]) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return Some(trimmed.to_string());
            }
        }
    }

    let raw = read_string_field(input, &["input", "selector", "name"])?;
    infer_uia_app_hint_details(&raw).map(|(canonical, _)| canonical.to_string())
}

fn strip_trailing_app_hint(target: &str, aliases: &[&str]) -> String {
    let lower = target.to_lowercase();
    for alias in aliases {
        for prefix in [
            " in the ", " in ", " on the ", " on ", " inside ", " within ",
        ] {
            let suffix = format!("{}{}", prefix, alias);
            if lower.ends_with(&suffix) {
                let end = target.len().saturating_sub(suffix.len());
                return target[..end].trim().to_string();
            }
        }
    }
    target.trim().to_string()
}

fn extract_uia_selector_target(input: &serde_json::Value, action: &str) -> Option<String> {
    if let Some(target) = extract_os_tool_target(input, action) {
        return Some(target);
    }

    let raw = read_string_field(input, &["input"])?;
    let lower = raw.to_lowercase();

    match action {
        "click" => {
            for prefix in ["click ", "tap "] {
                if let Some(rest) = lower.strip_prefix(prefix) {
                    return Some(raw[raw.len() - rest.len()..].trim().to_string());
                }
            }
        }
        "invoke_element" => {
            for prefix in [
                "invoke ",
                "click ",
                "press ",
                "toggle ",
                "expand ",
                "collapse ",
                "select ",
            ] {
                if let Some(rest) = lower.strip_prefix(prefix) {
                    return Some(raw[raw.len() - rest.len()..].trim().to_string());
                }
            }
        }
        "focus" => {
            if let Some(rest) = lower.strip_prefix("focus ") {
                return Some(raw[raw.len() - rest.len()..].trim().to_string());
            }
        }
        "type_text" => {
            for marker in [" into the ", " into ", " in the ", " in "] {
                if let Some(index) = lower.find(marker) {
                    let target = raw[index + marker.len()..].trim();
                    if !target.is_empty() {
                        return Some(target.to_string());
                    }
                }
            }
        }
        _ => {}
    }

    None
}

fn build_uia_selector_object(input: &serde_json::Value) -> serde_json::Value {
    let mut selector = serde_json::Map::new();

    for (src, dst) in [
        ("selector", "selector"),
        ("name", "name"),
        ("automation_id", "automation_id"),
        ("automationid", "automation_id"),
        ("class_name", "class_name"),
        ("classname", "class_name"),
        ("control_type", "control_type"),
        ("controltype", "control_type"),
    ] {
        if let Some(value) = read_string_field(input, &[src]) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                selector.insert(
                    dst.to_string(),
                    serde_json::Value::String(trimmed.to_string()),
                );
            }
        }
    }

    if let Some(app_hint) = infer_uia_app_hint(input) {
        selector.insert("app_hint".to_string(), serde_json::Value::String(app_hint));
    }

    if !selector.contains_key("name") && !selector.contains_key("selector") {
        let action = infer_os_tool_action(input);
        if let Some(target) = extract_uia_selector_target(input, &action)
            .or_else(|| read_string_field(input, &["input"]))
        {
            let trimmed_value = infer_uia_app_hint_details(target.trim())
                .map(|(_, aliases)| strip_trailing_app_hint(target.trim(), aliases))
                .unwrap_or_else(|| target.trim().to_string());
            let trimmed = trimmed_value.trim();
            if !trimmed.is_empty() {
                let cleaned_name = clean_uia_selector_name(trimmed);
                if !cleaned_name.is_empty() && cleaned_name.len() < trimmed.len() {
                    selector.insert("name".to_string(), serde_json::Value::String(cleaned_name));
                } else {
                    selector.insert(
                        "selector".to_string(),
                        serde_json::Value::String(trimmed.to_string()),
                    );
                }
                if let Some(control_type) = infer_uia_control_type(trimmed) {
                    selector.insert(
                        "control_type".to_string(),
                        serde_json::Value::String(control_type.to_string()),
                    );
                }
            }
        }
    }

    serde_json::Value::Object(selector)
}

fn has_uia_selector(input: &serde_json::Value) -> bool {
    build_uia_selector_object(input)
        .as_object()
        .map(|selector| !selector.is_empty())
        .unwrap_or(false)
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
        let prefixes = ["press ", "hotkey ", "shortcut ", "key combo ", "combo "];
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

fn with_enigo() -> Result<Enigo, DynamicToolError> {
    Enigo::new(&Settings::default()).map_err(|err| DynamicToolError::ExecutionFailed {
        tool: "os_tool".to_string(),
        details: format!("Failed to initialize native desktop input: {}", err),
    })
}

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
Add-Type -AssemblyName UIAutomationClient
$hwnd = [OverseerWinApi]::GetForegroundWindow()
$activeTitle = ""
$activePid = 0
if ($hwnd -ne [IntPtr]::Zero) {
    $sb = New-Object System.Text.StringBuilder 1024
    [void][OverseerWinApi]::GetWindowText($hwnd, $sb, $sb.Capacity)
    $activeTitle = $sb.ToString()
    [void][OverseerWinApi]::GetWindowThreadProcessId($hwnd, [ref]$activePid)
}
$activeElement = $null
try {
    if ($hwnd -ne [IntPtr]::Zero) {
        $activeElement = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
    }
} catch {}
$focusedElement = $null
try {
    $focusedElement = [System.Windows.Automation.AutomationElement]::FocusedElement
} catch {}
function Convert-UiElement($element) {
    if ($null -eq $element) { return $null }
    $rect = $null
    try { $rect = $element.Current.BoundingRectangle } catch {}
    [pscustomobject]@{
        name = try { $element.Current.Name } catch { $null }
        automation_id = try { $element.Current.AutomationId } catch { $null }
        class_name = try { $element.Current.ClassName } catch { $null }
        control_type = try { $element.Current.ControlType.ProgrammaticName } catch { $null }
        process_id = try { $element.Current.ProcessId } catch { $null }
        is_enabled = try { $element.Current.IsEnabled } catch { $null }
        has_keyboard_focus = try { $element.Current.HasKeyboardFocus } catch { $null }
        bounding_rectangle = if ($rect) {
            [pscustomobject]@{
                left = [int]$rect.Left
                top = [int]$rect.Top
                width = [int]$rect.Width
                height = [int]$rect.Height
            }
        } else { $null }
    }
}
$uiaElements = @()
if ($activeElement -ne $null) {
    try {
        $descendants = $activeElement.FindAll(
            [System.Windows.Automation.TreeScope]::Descendants,
            [System.Windows.Automation.Condition]::TrueCondition
        )
        $max = [Math]::Min($descendants.Count, 40)
        for ($i = 0; $i -lt $max; $i++) {
            $uiaElements += Convert-UiElement($descendants.Item($i))
        }
    } catch {}
}
$windows = Get-Process |
    Where-Object { $_.MainWindowTitle -and $_.MainWindowTitle.Trim().Length -gt 0 } |
    Select-Object -First 12 ProcessName, Id, MainWindowTitle
[pscustomobject]@{
    active_window_title = $activeTitle
    active_pid = $activePid
    visible_windows = $windows
    accessibility = [pscustomobject]@{
        active_window = Convert-UiElement($activeElement)
        focused_element = Convert-UiElement($focusedElement)
        elements = $uiaElements
        element_count = @($uiaElements).Count
    }
} | ConvertTo-Json -Depth 6 -Compress
"#;

    run_powershell_hidden(script, tool_name).await
}

#[cfg(windows)]
async fn windows_accessibility_state_json(
    tool_name: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    let selector_json = build_uia_selector_object(input).to_string();
    let escaped_selector_json = escape_powershell_single_quoted(&selector_json);
    let script = format!(
        r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class OverseerWinApi {{
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
}}
"@
Add-Type -AssemblyName UIAutomationClient
$selector = '{selector_json}' | ConvertFrom-Json
$hwnd = [OverseerWinApi]::GetForegroundWindow()
$root = $null
try {{
    if ($hwnd -ne [IntPtr]::Zero) {{
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
    }}
}} catch {{}}
if ($null -eq $root) {{
    $root = [System.Windows.Automation.AutomationElement]::RootElement
}}
function Convert-UiElement($element) {{
    if ($null -eq $element) {{ return $null }}
    $rect = $null
    try {{ $rect = $element.Current.BoundingRectangle }} catch {{}}
    [pscustomobject]@{{
        name = try {{ $element.Current.Name }} catch {{ $null }}
        automation_id = try {{ $element.Current.AutomationId }} catch {{ $null }}
        class_name = try {{ $element.Current.ClassName }} catch {{ $null }}
        control_type = try {{ $element.Current.ControlType.ProgrammaticName }} catch {{ $null }}
        process_id = try {{ $element.Current.ProcessId }} catch {{ $null }}
        is_enabled = try {{ $element.Current.IsEnabled }} catch {{ $null }}
        has_keyboard_focus = try {{ $element.Current.HasKeyboardFocus }} catch {{ $null }}
        bounding_rectangle = if ($rect) {{
            [pscustomobject]@{{
                left = [int]$rect.Left
                top = [int]$rect.Top
                width = [int]$rect.Width
                height = [int]$rect.Height
            }}
        }} else {{ $null }}
    }}
}}
function Matches-Selector($element, $selector) {{
    if ($null -eq $element) {{ return $false }}
    $name = (try {{ $element.Current.Name }} catch {{ '' }})
    $automationId = (try {{ $element.Current.AutomationId }} catch {{ '' }})
    $className = (try {{ $element.Current.ClassName }} catch {{ '' }})
    $controlType = (try {{ $element.Current.ControlType.ProgrammaticName }} catch {{ '' }})
    if ($selector.name -and $name -notlike ('*' + $selector.name + '*')) {{ return $false }}
    if ($selector.automation_id -and $automationId -notlike ('*' + $selector.automation_id + '*')) {{ return $false }}
    if ($selector.class_name -and $className -notlike ('*' + $selector.class_name + '*')) {{ return $false }}
    if ($selector.control_type -and $controlType -notlike ('*' + $selector.control_type + '*')) {{ return $false }}
    if ($selector.selector) {{
        $needle = [string]$selector.selector
        if ($name -notlike ('*' + $needle + '*') -and
            $automationId -notlike ('*' + $needle + '*') -and
            $className -notlike ('*' + $needle + '*') -and
            $controlType -notlike ('*' + $needle + '*')) {{
            return $false
        }}
    }}
    return $true
}}
function Matches-AppHint($element, $selector) {{
    if ($null -eq $element -or -not $selector.app_hint) {{ return $false }}
    $hint = [string]$selector.app_hint
    $name = (try {{ $element.Current.Name }} catch {{ '' }})
    $className = (try {{ $element.Current.ClassName }} catch {{ '' }})
    $processName = ''
    try {{
        $processId = $element.Current.ProcessId
        if ($processId -gt 0) {{
            $processName = (Get-Process -Id $processId -ErrorAction Stop).ProcessName
        }}
    }} catch {{}}
    return $name -like ('*' + $hint + '*') -or
        $className -like ('*' + $hint + '*') -or
        $processName -like ('*' + $hint + '*')
}}
$preferredRoot = $null
$desktopRoot = [System.Windows.Automation.AutomationElement]::RootElement
if ($selector.app_hint -and $null -ne $desktopRoot) {{
    try {{
        $topWindows = $desktopRoot.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
        for ($i = 0; $i -lt $topWindows.Count; $i++) {{
            $candidateWindow = $topWindows.Item($i)
            if (Matches-AppHint $candidateWindow $selector) {{
                $preferredRoot = $candidateWindow
                break
            }}
        }}
    }} catch {{}}
}}
$focused = $null
try {{ $focused = [System.Windows.Automation.AutomationElement]::FocusedElement }} catch {{}}
$searchRoots = New-Object System.Collections.Generic.List[System.Windows.Automation.AutomationElement]
if ($null -ne $preferredRoot) {{
    $searchRoots.Add($preferredRoot)
}}
if ($null -ne $root) {{
    try {{
        if ($null -eq $preferredRoot -or $preferredRoot.Current.NativeWindowHandle -ne $root.Current.NativeWindowHandle) {{
            $searchRoots.Add($root)
        }}
    }} catch {{
        $searchRoots.Add($root)
    }}
}}
if ($null -ne $desktopRoot) {{
    $rootHandle = $null
    $desktopHandle = $null
    try {{ $rootHandle = $root.Current.NativeWindowHandle }} catch {{}}
    try {{ $desktopHandle = $desktopRoot.Current.NativeWindowHandle }} catch {{}}
    if ($rootHandle -ne $desktopHandle) {{
        $searchRoots.Add($desktopRoot)
    }}
}}
$found = @()
$searchScope = 'active_window'
foreach ($searchRoot in $searchRoots) {{
    try {{
        $all = $searchRoot.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        $max = [Math]::Min($all.Count, 180)
        for ($i = 0; $i -lt $max; $i++) {{
            $candidate = $all.Item($i)
            if (Matches-Selector $candidate $selector) {{
                $found += Convert-UiElement($candidate)
            }}
        }}
        if (@($found).Count -gt 0) {{
            try {{
                if ($null -ne $preferredRoot -and $searchRoot.Current.NativeWindowHandle -eq $preferredRoot.Current.NativeWindowHandle) {{
                    $searchScope = 'preferred_app_window'
                }} elseif ($searchRoot.Current.NativeWindowHandle -eq $desktopRoot.Current.NativeWindowHandle) {{
                    $searchScope = 'desktop_root'
                }}
            }} catch {{}}
            break
        }}
    }} catch {{}}
}}
[pscustomobject]@{{
    action = 'inspect_accessibility'
    selector = $selector
    focused_element = Convert-UiElement($focused)
    search_scope = $searchScope
    matches = $found | Select-Object -First 20
    match_count = @($found).Count
}} | ConvertTo-Json -Depth 6 -Compress
"#,
        selector_json = escaped_selector_json
    );

    run_powershell_hidden(&script, tool_name).await
}

#[cfg(windows)]
async fn windows_uia_interaction_json(
    tool_name: &str,
    action: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    let selector_json = build_uia_selector_object(input).to_string();
    let escaped_selector_json = escape_powershell_single_quoted(&selector_json);
    let escaped_action = escape_powershell_single_quoted(action);
    let interaction_text = if action == "type_text" {
        extract_text_value(input).unwrap_or_default()
    } else {
        String::new()
    };
    let escaped_text = escape_powershell_single_quoted(&interaction_text);
    let script = format!(
        r#"
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class OverseerWinApi {{
    [DllImport("user32.dll")]
    public static extern IntPtr GetForegroundWindow();
}}
"@
Add-Type -AssemblyName UIAutomationClient
$selector = '{selector_json}' | ConvertFrom-Json
$action = '{action_name}'
$text = '{text_value}'
$hwnd = [OverseerWinApi]::GetForegroundWindow()
$root = $null
try {{
    if ($hwnd -ne [IntPtr]::Zero) {{
        $root = [System.Windows.Automation.AutomationElement]::FromHandle($hwnd)
    }}
}} catch {{}}
if ($null -eq $root) {{
    throw 'No active window UI Automation root available.'
}}
function Convert-UiElement($element) {{
    if ($null -eq $element) {{ return $null }}
    $rect = $null
    try {{ $rect = $element.Current.BoundingRectangle }} catch {{}}
    [pscustomobject]@{{
        name = try {{ $element.Current.Name }} catch {{ $null }}
        automation_id = try {{ $element.Current.AutomationId }} catch {{ $null }}
        class_name = try {{ $element.Current.ClassName }} catch {{ $null }}
        control_type = try {{ $element.Current.ControlType.ProgrammaticName }} catch {{ $null }}
        process_id = try {{ $element.Current.ProcessId }} catch {{ $null }}
        is_enabled = try {{ $element.Current.IsEnabled }} catch {{ $null }}
        has_keyboard_focus = try {{ $element.Current.HasKeyboardFocus }} catch {{ $null }}
        bounding_rectangle = if ($rect) {{
            [pscustomobject]@{{
                left = [int]$rect.Left
                top = [int]$rect.Top
                width = [int]$rect.Width
                height = [int]$rect.Height
            }}
        }} else {{ $null }}
    }}
}}
function Matches-Selector($element, $selector) {{
    if ($null -eq $element) {{ return $false }}
    $name = (try {{ $element.Current.Name }} catch {{ '' }})
    $automationId = (try {{ $element.Current.AutomationId }} catch {{ '' }})
    $className = (try {{ $element.Current.ClassName }} catch {{ '' }})
    $controlType = (try {{ $element.Current.ControlType.ProgrammaticName }} catch {{ '' }})
    if ($selector.name -and $name -notlike ('*' + $selector.name + '*')) {{ return $false }}
    if ($selector.automation_id -and $automationId -notlike ('*' + $selector.automation_id + '*')) {{ return $false }}
    if ($selector.class_name -and $className -notlike ('*' + $selector.class_name + '*')) {{ return $false }}
    if ($selector.control_type -and $controlType -notlike ('*' + $selector.control_type + '*')) {{ return $false }}
    if ($selector.selector) {{
        $needle = [string]$selector.selector
        if ($name -notlike ('*' + $needle + '*') -and
            $automationId -notlike ('*' + $needle + '*') -and
            $className -notlike ('*' + $needle + '*') -and
            $controlType -notlike ('*' + $needle + '*')) {{
            return $false
        }}
    }}
    return $true
}}
function Matches-AppHint($element, $selector) {{
    if ($null -eq $element -or -not $selector.app_hint) {{ return $false }}
    $hint = [string]$selector.app_hint
    $name = (try {{ $element.Current.Name }} catch {{ '' }})
    $className = (try {{ $element.Current.ClassName }} catch {{ '' }})
    $processName = ''
    try {{
        $processId = $element.Current.ProcessId
        if ($processId -gt 0) {{
            $processName = (Get-Process -Id $processId -ErrorAction Stop).ProcessName
        }}
    }} catch {{}}
    return $name -like ('*' + $hint + '*') -or
        $className -like ('*' + $hint + '*') -or
        $processName -like ('*' + $hint + '*')
}}
function Get-DiagnosticCandidates($searchRoot, $selector) {{
    $candidates = @()
    try {{
        $all = $searchRoot.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        $max = [Math]::Min($all.Count, 160)
        for ($i = 0; $i -lt $max; $i++) {{
            $candidate = $all.Item($i)
            $converted = Convert-UiElement($candidate)
            if ($null -eq $converted) {{ continue }}
            $score = 0
            if ($selector.name -and $converted.name -like ('*' + $selector.name + '*')) {{ $score += 6 }}
            if ($selector.selector -and $converted.name -like ('*' + $selector.selector + '*')) {{ $score += 4 }}
            if ($selector.automation_id -and $converted.automation_id -like ('*' + $selector.automation_id + '*')) {{ $score += 5 }}
            if ($selector.class_name -and $converted.class_name -like ('*' + $selector.class_name + '*')) {{ $score += 3 }}
            if ($selector.control_type -and $converted.control_type -like ('*' + $selector.control_type + '*')) {{ $score += 4 }}
            if ($score -gt 0 -or ($candidates.Count -lt 8 -and $converted.name)) {{
                $converted | Add-Member -NotePropertyName score -NotePropertyValue $score
                $candidates += $converted
            }}
        }}
    }} catch {{}}
    return $candidates | Sort-Object -Property score -Descending | Select-Object -First 8
}}
$preferredRoot = $null
$desktopRoot = [System.Windows.Automation.AutomationElement]::RootElement
if ($selector.app_hint -and $null -ne $desktopRoot) {{
    try {{
        $topWindows = $desktopRoot.FindAll([System.Windows.Automation.TreeScope]::Children, [System.Windows.Automation.Condition]::TrueCondition)
        for ($i = 0; $i -lt $topWindows.Count; $i++) {{
            $candidateWindow = $topWindows.Item($i)
            if (Matches-AppHint $candidateWindow $selector) {{
                $preferredRoot = $candidateWindow
                break
            }}
        }}
    }} catch {{}}
}}
$searchRoots = New-Object System.Collections.Generic.List[System.Windows.Automation.AutomationElement]
if ($null -ne $preferredRoot) {{
    $searchRoots.Add($preferredRoot)
}}
if ($null -ne $root) {{
    try {{
        if ($null -eq $preferredRoot -or $preferredRoot.Current.NativeWindowHandle -ne $root.Current.NativeWindowHandle) {{
            $searchRoots.Add($root)
        }}
    }} catch {{
        $searchRoots.Add($root)
    }}
}}
if ($null -ne $desktopRoot) {{
    $rootHandle = $null
    $desktopHandle = $null
    try {{ $rootHandle = $root.Current.NativeWindowHandle }} catch {{}}
    try {{ $desktopHandle = $desktopRoot.Current.NativeWindowHandle }} catch {{}}
    if ($rootHandle -ne $desktopHandle) {{
        $searchRoots.Add($desktopRoot)
    }}
}}
$match = $null
$searchScope = 'active_window'
foreach ($searchRoot in $searchRoots) {{
    try {{
        $all = $searchRoot.FindAll([System.Windows.Automation.TreeScope]::Descendants, [System.Windows.Automation.Condition]::TrueCondition)
        $max = [Math]::Min($all.Count, 220)
        for ($i = 0; $i -lt $max; $i++) {{
            $candidate = $all.Item($i)
            if (Matches-Selector $candidate $selector) {{
                $match = $candidate
                try {{
                    if ($null -ne $preferredRoot -and $searchRoot.Current.NativeWindowHandle -eq $preferredRoot.Current.NativeWindowHandle) {{
                        $searchScope = 'preferred_app_window'
                    }} elseif ($searchRoot.Current.NativeWindowHandle -eq $desktopRoot.Current.NativeWindowHandle) {{
                        $searchScope = 'desktop_root'
                    }}
                }} catch {{}}
                break
            }}
        }}
    }} catch {{}}
    if ($null -ne $match) {{
        break
    }}
}}
if ($null -eq $match) {{
    $focused = $null
    try {{ $focused = [System.Windows.Automation.AutomationElement]::FocusedElement }} catch {{}}
    $activeWindow = Convert-UiElement($root)
    $candidateSearchRoot = if ($null -ne $preferredRoot) {{ $preferredRoot }} elseif ($null -ne $root) {{ $root }} else {{ $desktopRoot }}
    $blocked = [pscustomobject]@{{
        action = $action
        selector = $selector
        app_hint = $selector.app_hint
        search_scope = if ($null -ne $preferredRoot) {{ 'preferred_app_window' }} else {{ 'active_window' }}
        active_window = $activeWindow
        focused_element = Convert-UiElement($focused)
        candidates = @(Get-DiagnosticCandidates $candidateSearchRoot $selector)
        retry_hint = if ($selector.app_hint) {{
            'The requested control was not found in the preferred app window. Try a more specific control name, or focus the correct window first.'
        }} else {{
            'Try inspecting accessibility for the current window, focusing the intended app first, or using a more specific control name.'
        }}
    }}
    throw ('UIA_BLOCKED:' + ($blocked | ConvertTo-Json -Depth 6 -Compress))
}}
$method = 'focus'
$verified = $false
$verification = 'not_checked'
$postFocused = $null
$postElement = $null
function Same-UiElement($left, $right) {{
    if ($null -eq $left -or $null -eq $right) {{ return $false }}
    try {{
        if ($left.Current.NativeWindowHandle -ne 0 -and $left.Current.NativeWindowHandle -eq $right.Current.NativeWindowHandle) {{
            return $true
        }}
    }} catch {{}}
    $leftName = (try {{ $left.Current.Name }} catch {{ '' }})
    $rightName = (try {{ $right.Current.Name }} catch {{ '' }})
    $leftAuto = (try {{ $left.Current.AutomationId }} catch {{ '' }})
    $rightAuto = (try {{ $right.Current.AutomationId }} catch {{ '' }})
    $leftType = (try {{ $left.Current.ControlType.ProgrammaticName }} catch {{ '' }})
    $rightType = (try {{ $right.Current.ControlType.ProgrammaticName }} catch {{ '' }})
    return $leftName -eq $rightName -and $leftAuto -eq $rightAuto -and $leftType -eq $rightType
}}
if ($action -eq 'invoke_element') {{
    $invoked = $false
    $toggleBefore = $null
    $toggleAfter = $null
    $expandBefore = $null
    $expandAfter = $null
    try {{
        $pattern = $match.GetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern)
        if ($pattern) {{
            $pattern.Invoke()
            $method = 'invoke_pattern'
            $invoked = $true
        }}
    }} catch {{}}
    if (-not $invoked) {{
        try {{
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
            if ($pattern) {{
                $pattern.Select()
                $method = 'selection_item_pattern'
                $invoked = $true
            }}
        }} catch {{}}
    }}
    if (-not $invoked) {{
        try {{
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern)
            if ($pattern) {{
                $toggleBefore = [string]$pattern.Current.ToggleState
                $pattern.Toggle()
                $toggleAfter = [string]$pattern.Current.ToggleState
                $method = 'toggle_pattern'
                $invoked = $true
            }}
        }} catch {{}}
    }}
    if (-not $invoked) {{
        try {{
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern)
            if ($pattern) {{
                $expandBefore = [string]$pattern.Current.ExpandCollapseState
                if ($expandBefore -eq 'Collapsed') {{
                    $pattern.Expand()
                }} else {{
                    $pattern.Collapse()
                }}
                $expandAfter = [string]$pattern.Current.ExpandCollapseState
                $method = 'expand_collapse_pattern'
                $invoked = $true
            }}
        }} catch {{}}
    }}
    if (-not $invoked) {{
        try {{
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.LegacyIAccessiblePattern]::Pattern)
            if ($pattern) {{
                $pattern.DoDefaultAction()
                $method = 'legacy_default_action'
                $invoked = $true
            }}
        }} catch {{}}
    }}
    if (-not $invoked) {{
        $match.SetFocus()
        $method = 'focus_fallback'
    }}
    try {{ $postFocused = [System.Windows.Automation.AutomationElement]::FocusedElement }} catch {{}}
    $postElement = $match
    if ($method -eq 'selection_item_pattern') {{
        try {{
            $pattern = $match.GetCurrentPattern([System.Windows.Automation.SelectionItemPattern]::Pattern)
            $verified = [bool]$pattern.Current.IsSelected
            $verification = 'selection_item_selected'
        }} catch {{}}
    }} elseif ($method -eq 'toggle_pattern') {{
        $verified = $toggleBefore -ne $toggleAfter -and $null -ne $toggleAfter
        $verification = if ($verified) {{ 'toggle_state_changed' }} else {{ 'toggle_state_unknown' }}
    }} elseif ($method -eq 'expand_collapse_pattern') {{
        $verified = $expandBefore -ne $expandAfter -and $null -ne $expandAfter
        $verification = if ($verified) {{ 'expand_collapse_state_changed' }} else {{ 'expand_collapse_state_unknown' }}
    }} elseif ($method -eq 'invoke_pattern' -or $method -eq 'legacy_default_action' -or $method -eq 'focus_fallback') {{
        $verified = Same-UiElement $match $postFocused
        $verification = if ($verified) {{ 'focused_after_invoke' }} else {{ 'invoke_not_confirmed' }}
    }}
}} elseif ($action -eq 'type_text') {{
    $typed = $false
    $typedValue = $null
    try {{
        $pattern = $match.GetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern)
        if ($pattern) {{
            $pattern.SetValue($text)
            $typedValue = [string]$pattern.Current.Value
            $method = 'value_pattern'
            $typed = $true
        }}
    }} catch {{}}
    if (-not $typed) {{
        try {{
            $match.SetFocus()
            $method = 'focus_ready'
        }} catch {{
            throw 'Matched UI Automation element could not be prepared for text input.'
        }}
    }}
    try {{ $postFocused = [System.Windows.Automation.AutomationElement]::FocusedElement }} catch {{}}
    $postElement = $match
    if ($method -eq 'value_pattern') {{
        $verified = $typedValue -eq $text
        $verification = if ($verified) {{ 'value_pattern_roundtrip' }} else {{ 'value_pattern_unconfirmed' }}
    }} else {{
        $verified = Same-UiElement $match $postFocused
        $verification = if ($verified) {{ 'focused_for_keyboard_typing' }} else {{ 'keyboard_typing_unconfirmed' }}
    }}
}} else {{
    $match.SetFocus()
    $method = 'focus'
    try {{ $postFocused = [System.Windows.Automation.AutomationElement]::FocusedElement }} catch {{}}
    $postElement = $match
    $verified = Same-UiElement $match $postFocused
    $verification = if ($verified) {{ 'focused_element_confirmed' }} else {{ 'focus_not_confirmed' }}
}}
[pscustomobject]@{{
    action = $action
    selector = $selector
    search_scope = $searchScope
    method = $method
    element = Convert-UiElement($match)
    post_action_focused_element = Convert-UiElement($postFocused)
    post_action_element = Convert-UiElement($postElement)
    verification = $verification
    verified = $verified
    text_length = if ($action -eq 'type_text') {{ $text.Length }} else {{ $null }}
    ok = $true
}} | ConvertTo-Json -Depth 6 -Compress
"#,
        selector_json = escaped_selector_json,
        action_name = escaped_action,
        text_value = escaped_text
    );

    run_powershell_hidden(&script, tool_name).await
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
async fn windows_find_files_json(tool_name: &str, query: &str) -> Result<String, DynamicToolError> {
    if let Some(results) = search_workspace_files_fast(query).await {
        let cwd = std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().to_string())
            .into_iter()
            .collect::<Vec<_>>();

        return Ok(json!({
            "action": "find_files",
            "query": query,
            "roots": cwd,
            "source": "workspace_fast_path",
            "results": results,
        })
        .to_string());
    }

    let escaped_query = escape_powershell_single_quoted(query);
    let script = format!(
        r#"
$query = '{query}'
$home = [Environment]::GetFolderPath('UserProfile')
$cwd = [System.IO.Directory]::GetCurrentDirectory()
$roots = @(
    $cwd,
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

$seen = New-Object 'System.Collections.Generic.HashSet[string]'
$matches = New-Object System.Collections.Generic.List[object]
foreach ($root in $roots) {{
    try {{
        foreach ($token in $tokens) {{
            Get-ChildItem -Path $root -Recurse -File -Filter "*$token*" -ErrorAction SilentlyContinue |
                Select-Object -First 25 |
                ForEach-Object {{
                $normalized = $_.FullName.ToLower()
                if (-not $seen.Add($normalized)) {{
                    return
                }}
                $full = $_.FullName.ToLower()
                $name = $_.Name.ToLower()
                $score = 0
                if ($name -eq $query.ToLower()) {{ $score += 200 }}
                foreach ($token in $tokens) {{
                    if ($name -eq $token) {{ $score += 80 }}
                    if ($name -like "*$token*") {{ $score += 30 }}
                    if ($full -like "*$token*") {{ $score += 10 }}
                }}
                if ($full -like "*\overseer\src\*") {{ $score += 40 }}
                if ($full -like "*\desktop\*") {{ $score += 5 }}

                $matches.Add([pscustomobject]@{{
                    name = $_.Name
                    path = $_.FullName
                    directory = $_.DirectoryName
                    extension = $_.Extension
                    score = $score
                }})
                }}

            if ($matches.Count -ge 40) {{ break }}
        }}
    }} catch {{}}
    if ($matches.Count -ge 40) {{ break }}
}}

if ($matches.Count -eq 0) {{
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
                Select-Object -First 20 |
                ForEach-Object {{
                    $normalized = $_.FullName.ToLower()
                    if (-not $seen.Add($normalized)) {{
                        return
                    }}
                    $matches.Add([pscustomobject]@{{
                        name = $_.Name
                        path = $_.FullName
                        directory = $_.DirectoryName
                        extension = $_.Extension
                        score = 1
                    }})
                }}
        }} catch {{}}
        if ($matches.Count -ge 20) {{ break }}
    }}
}}

$results = $matches |
    Sort-Object -Property @{{Expression='score';Descending=$true}}, @{{Expression='path';Descending=$false}} |
    Select-Object -First 20 name, path, directory, extension

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
async fn windows_read_file_json(tool_name: &str, path: &str) -> Result<String, DynamicToolError> {
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
async fn windows_open_file_json(tool_name: &str, path: &str) -> Result<String, DynamicToolError> {
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
$windows = @(Get-Process |
    Where-Object { $browserNames -contains $_.ProcessName.ToLower() -and $_.MainWindowTitle -and $_.MainWindowTitle.Trim().Length -gt 0 } |
    Select-Object -First 12 ProcessName, Id, MainWindowTitle)
[pscustomobject]@{
    action = 'browser_state'
    browser_windows = $windows
    browser_count = @($windows).Count
} | ConvertTo-Json -Depth 4 -Compress
"#;

    run_powershell_hidden(script, tool_name).await
}

#[cfg(not(windows))]
async fn capture_linux_screenshot_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("overseer_capture_{}.png", uuid::Uuid::new_v4()));
    let path_str = temp_path.to_string_lossy().to_string();

    let mut command_args = vec![
        ("scrot", vec![path_str.clone()]),
        ("gnome-screenshot", vec!["-f".to_string(), path_str.clone()]),
        (
            "import",
            vec!["-window".to_string(), "root".to_string(), path_str.clone()],
        ),
    ];

    let mut captured = false;
    for (cmd, args) in command_args {
        if Command::new(cmd).arg("--version").output().await.is_ok() {
            let output = Command::new(cmd).args(args).output().await.map_err(|e| {
                DynamicToolError::ExecutionFailed {
                    tool: tool_name.to_string(),
                    details: e.to_string(),
                }
            })?;
            if output.status.success() {
                captured = true;
                break;
            }
        }
    }

    if !captured {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: "No supported Linux screenshot utility found (tried scrot, gnome-screenshot, import).".to_string(),
        });
    }

    let mut width = 1920;
    let mut height = 1080;
    if let Ok(output) = Command::new("xdpyinfo").output().await {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            if line.contains("dimensions:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let dims: Vec<&str> = parts[1].split('x').collect();
                    if dims.len() >= 2 {
                        width = dims[0].parse().unwrap_or(1920);
                        height = dims[1].parse().unwrap_or(1080);
                    }
                }
            }
        }
    }

    Ok(json!({
        "action": "screenshot",
        "path": path_str,
        "width": width,
        "height": height,
        "summary": format!("Captured a Linux desktop screenshot at {}x{}.", width, height)
    })
    .to_string())
}

#[cfg(not(windows))]
async fn capture_macos_screenshot_json(tool_name: &str) -> Result<String, DynamicToolError> {
    let mut temp_path = std::env::temp_dir();
    temp_path.push(format!("overseer_capture_{}.png", uuid::Uuid::new_v4()));
    let path_str = temp_path.to_string_lossy().to_string();

    // Use macOS screencapture utility (-x for no sound)
    let output = Command::new("screencapture")
        .arg("-x")
        .arg(&path_str)
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run screencapture: {}", e),
        })?;

    if !output.status.success() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: String::from_utf8_lossy(&output.stderr).trim().to_string(),
        });
    }

    // Get resolution via system_profiler
    let mut width = 1440;
    let mut height = 900;
    if let Ok(res_out) = Command::new("system_profiler")
        .arg("SPDisplaysDataType")
        .output()
        .await
    {
        let stdout = String::from_utf8_lossy(&res_out.stdout);
        for line in stdout.lines() {
            if line.contains("Resolution:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    let dims: Vec<&str> = parts[1].split('x').collect();
                    if dims.len() >= 2 {
                        width = dims[0].parse().unwrap_or(1440);
                        height = dims[1].parse().unwrap_or(900);
                    }
                }
            }
        }
    }

    Ok(json!({
        "action": "screenshot",
        "path": path_str,
        "width": width,
        "height": height,
        "summary": format!("Captured a macOS desktop screenshot at {}x{}.", width, height)
    })
    .to_string())
}

#[cfg(not(windows))]
async fn capture_windows_screenshot_json(tool_name: &str) -> Result<String, DynamicToolError> {
    if cfg!(target_os = "macos") {
        capture_macos_screenshot_json(tool_name).await
    } else {
        capture_linux_screenshot_json(tool_name).await
    }
}

#[cfg(not(windows))]
async fn windows_process_list_json(tool_name: &str) -> Result<String, DynamicToolError> {
    run_command_simple("ps -eo pid,comm,%cpu,rss --sort=-%cpu", tool_name).await
}

#[cfg(not(windows))]
async fn windows_find_files_json(tool_name: &str, query: &str) -> Result<String, DynamicToolError> {
    if let Some(results) = search_workspace_files_fast(query).await {
        let cwd = std::env::current_dir()
            .ok()
            .map(|path| path.to_string_lossy().to_string())
            .into_iter()
            .collect::<Vec<_>>();

        return Ok(json!({
            "action": "find_files",
            "query": query,
            "roots": cwd,
            "source": "workspace_fast_path",
            "results": results,
        })
        .to_string());
    }

    let home = std::env::var("HOME").unwrap_or_default();
    let mut roots = vec![std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))];
    for candidate in [
        format!("{}/Desktop", home),
        format!("{}/Documents", home),
        format!("{}/Downloads", home),
        format!("{}/OneDrive", home),
    ] {
        let path = std::path::PathBuf::from(candidate);
        if path.exists() {
            roots.push(path);
        }
    }
    roots.sort();
    roots.dedup();

    let lower_query = query.to_lowercase();
    let mut tokens: Vec<String> = lower_query
        .split(|c: char| c.is_whitespace() || matches!(c, '?' | '.' | ',' | ':' | ';' | '\\' | '/'))
        .filter(|token| {
            token.len() >= 3
                && !matches!(
                    *token,
                    "find" | "path" | "file" | "files" | "pc" | "computer" | "my" | "the" | "in"
                )
        })
        .map(|token| token.to_string())
        .collect();

    if tokens.is_empty() {
        tokens.push(lower_query.clone());
    }

    let mut file_lines = Vec::new();
    if Command::new("rg").arg("--version").output().await.is_ok() {
        let mut cmd = Command::new("rg");
        cmd.arg("--files");
        for root in &roots {
            cmd.arg(root);
        }

        if let Ok(output) = cmd.output().await {
            file_lines.extend(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|line| line.to_string()),
            );
        }
    }

    if file_lines.is_empty() {
        let mut cmd = Command::new("find");
        for root in &roots {
            cmd.arg(root);
        }
        cmd.args(["-type", "f"]);

        let output = cmd
            .output()
            .await
            .map_err(|e| DynamicToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                details: e.to_string(),
            })?;

        file_lines.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.to_string()),
        );
    }

    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in file_lines {
        let path = line.trim();
        if path.is_empty() {
            continue;
        }

        let normalized = path.to_lowercase();
        if !seen.insert(normalized.clone()) {
            continue;
        }

        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(path)
            .to_string();
        let name_lower = name.to_lowercase();

        let matched = tokens
            .iter()
            .any(|token| name_lower.contains(token) || normalized.contains(token));
        if !matched {
            continue;
        }

        let mut score = 0;
        if name_lower == lower_query {
            score += 200;
        }
        for token in &tokens {
            if name_lower == *token {
                score += 80;
            }
            if name_lower.contains(token) {
                score += 30;
            }
            if normalized.contains(token) {
                score += 10;
            }
        }
        if normalized.contains("/overseer/src/") {
            score += 40;
        }
        if normalized.contains("/desktop/") {
            score += 5;
        }

        let directory = std::path::Path::new(path)
            .parent()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!(".{}", value))
            .unwrap_or_default();

        results.push(json!({
            "name": name,
            "path": path,
            "directory": directory,
            "extension": extension,
            "score": score
        }));
    }

    results.sort_by(|a, b| {
        let score_a = a.get("score").and_then(|value| value.as_i64()).unwrap_or(0);
        let score_b = b.get("score").and_then(|value| value.as_i64()).unwrap_or(0);
        score_b.cmp(&score_a).then_with(|| {
            a.get("path")
                .and_then(|value| value.as_str())
                .cmp(&b.get("path").and_then(|value| value.as_str()))
        })
    });
    results.truncate(20);

    Ok(json!({
        "action": "find_files",
        "query": query,
        "roots": roots.into_iter().map(|path| path.to_string_lossy().to_string()).collect::<Vec<_>>(),
        "source": "unix_native_search",
        "results": results,
    })
    .to_string())
}

#[cfg(not(windows))]
async fn windows_read_file_json(tool_name: &str, path: &str) -> Result<String, DynamicToolError> {
    let content =
        tokio::fs::read_to_string(path)
            .await
            .map_err(|e| DynamicToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                details: format!("Failed to read file '{}': {}", path, e),
            })?;

    let preview = if content.len() > 2000 {
        format!("{}...", &content[..2000])
    } else {
        content.clone()
    };

    Ok(json!({
        "action": "read_file",
        "path": path,
        "content_length": content.len(),
        "preview": preview,
        "summary": format!("Successfully read file '{}'. Captured {} bytes.", path, content.len())
    })
    .to_string())
}

#[cfg(not(windows))]
async fn windows_open_file_json(tool_name: &str, path: &str) -> Result<String, DynamicToolError> {
    let mut target_path = path.to_string();

    // Smart resolution for well-known Linux folders if the path doesn't exist directly
    if !cfg!(target_os = "macos") && !std::path::Path::new(&target_path).exists() {
        let lower = target_path.to_lowercase();
        let well_known = [
            "downloads",
            "documents",
            "pictures",
            "videos",
            "music",
            "desktop",
            "templates",
            "public",
        ];

        if well_known.iter().any(|&f| lower == f) {
            if let Ok(home) = std::env::var("HOME") {
                let capitalized = format!("{}{}", &lower[0..1].to_uppercase(), &lower[1..]);
                let home_path = std::path::PathBuf::from(home).join(&capitalized);
                if home_path.exists() {
                    target_path = home_path.to_string_lossy().to_string();
                }
            }
        }
    }

    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let status = Command::new(cmd)
        .arg(&target_path)
        .status()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to launch {} for '{}': {}", cmd, target_path, e),
        })?;

    if !status.success() {
        return Err(DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!(
                "{} failed with status {} for '{}'.",
                cmd, status, target_path
            ),
        });
    }

    Ok(json!({
        "action": "open_file",
        "path": target_path,
        "success": true,
        "summary": format!("Launched '{}' using {}.", target_path, cmd)
    })
    .to_string())
}

#[cfg(not(windows))]
async fn windows_browser_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    if cfg!(target_os = "macos") {
        return macos_browser_state_json(tool_name).await;
    }

    let output = Command::new("wmctrl")
        .arg("-l")
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run wmctrl: {}", e),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut browser_windows = Vec::new();

    let browser_keywords = [
        "chrome", "firefox", "chromium", "brave", "edge", "safari", "opera", "vivaldi",
    ];

    for line in stdout.lines() {
        let lower = line.to_lowercase();
        if browser_keywords.iter().any(|&k| lower.contains(k)) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            let title = if parts.len() >= 4 {
                parts[3..].join(" ")
            } else {
                line.to_string()
            };
            browser_windows.push(json!({
                "title": title,
                "ProcessName": parts.get(2).unwrap_or(&"unknown"),
                "Id": parts.first().unwrap_or(&"0")
            }));
        }
    }

    Ok(json!({
        "action": "browser_state",
        "browser_windows": browser_windows,
        "summary": format!("Detected {} browser window(s) on Linux.", browser_windows.len())
    })
    .to_string())
}

#[cfg(not(windows))]
async fn macos_browser_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    // macOS AppleScript to identify browser windows specifically
    let script = "tell application \"System Events\" to get name of every window of (every process whose background only is false and (name contains \"Chrome\" or name contains \"Firefox\" or name contains \"Safari\" or name contains \"Edge\" or name contains \"Brave\"))";

    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run osascript for browser state: {}", e),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let mut browser_windows = Vec::new();

    for name in stdout.split(", ") {
        if !name.is_empty() && name != "missing value" {
            browser_windows.push(json!({
                "title": name,
                "ProcessName": "unknown",
                "Id": "0"
            }));
        }
    }

    Ok(json!({
        "action": "browser_state",
        "browser_windows": browser_windows,
        "summary": format!("Detected {} browser window(s) on macOS.", browser_windows.len())
    })
    .to_string())
}

#[cfg(not(windows))]
async fn windows_accessibility_state_json(
    tool_name: &str,
    _input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    Ok(json!({
        "action": "inspect_accessibility",
        "focused_element": null,
        "summary": "Native accessibility inspection is not supported on Linux. Consider using browser tools."
    }).to_string())
}

#[cfg(not(windows))]
async fn windows_uia_interaction_json(
    tool_name: &str,
    _action: &str,
    _input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    Err(DynamicToolError::ExecutionFailed {
        tool: tool_name.to_string(),
        details: "Native UI Automation interaction is not supported on Linux.".to_string(),
    })
}

#[cfg(not(windows))]
async fn macos_desktop_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    // Get frontmost application name
    let active_app_out = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of first process whose frontmost is true")
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run osascript for active app: {}", e),
        })?;
    let active_title = String::from_utf8_lossy(&active_app_out.stdout)
        .trim()
        .to_string();

    // Get list of window titles for all visible processes
    let windows_out = Command::new("osascript")
        .arg("-e")
        .arg("tell application \"System Events\" to get name of every window of (every process whose background only is false)")
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: tool_name.to_string(),
            details: format!("Failed to run osascript for windows: {}", e),
        })?;
    let windows_raw = String::from_utf8_lossy(&windows_out.stdout)
        .trim()
        .to_string();

    // AppleScript returns a comma-separated list like "App1: WindowA, App1: WindowB, App2: WindowC"
    let visible_windows: Vec<serde_json::Value> = windows_raw
        .split(", ")
        .take(12)
        .filter(|s| !s.is_empty() && *s != "missing value")
        .map(|name| {
            json!({
                "ProcessName": "unknown",
                "Id": "0",
                "MainWindowTitle": name
            })
        })
        .collect();

    Ok(json!({
        "action": "list_windows",
        "visible_windows": visible_windows,
        "active_title": active_title,
        "summary": format!("macOS desktop reported. Active app: {}. Detected {} window(s).", active_title, visible_windows.len())
    }).to_string())
}

#[cfg(not(windows))]
async fn windows_desktop_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    if cfg!(target_os = "macos") {
        macos_desktop_state_json(tool_name).await
    } else {
        linux_desktop_state_json(tool_name).await
    }
}

#[cfg(not(windows))]
async fn linux_desktop_state_json(tool_name: &str) -> Result<String, DynamicToolError> {
    // Get active window title using xprop
    let active_window_id = run_command_simple("xprop -root _NET_ACTIVE_WINDOW", tool_name).await?;
    let mut active_title = String::new();
    if let Some(id) = active_window_id.split_whitespace().last() {
        let title_out =
            run_command_simple(&format!("xprop -id {} _NET_WM_NAME", id), tool_name).await?;
        if let Some(pos) = title_out.find(" = \"") {
            active_title = title_out[pos + 4..].trim_end_matches('"').to_string();
        } else {
            // Fallback to WM_NAME if _NET_WM_NAME is missing
            let title_out_fallback =
                run_command_simple(&format!("xprop -id {} WM_NAME", id), tool_name).await?;
            if let Some(pos) = title_out_fallback.find(" = \"") {
                active_title = title_out_fallback[pos + 4..]
                    .trim_end_matches('"')
                    .to_string();
            }
        }
    }

    // List windows using wmctrl
    let windows_raw = run_command_simple("wmctrl -l", tool_name)
        .await
        .unwrap_or_default();
    let visible_windows: Vec<serde_json::Value> = windows_raw
        .lines()
        .take(12)
        .map(|line| {
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
        })
        .collect();

    Ok(json!({
        "action": "list_windows",
        "visible_windows": visible_windows,
        "active_title": active_title,
        "summary": format!("Linux desktop reported. Active window: {}. Detected {} window(s).", active_title, visible_windows.len())
    }).to_string())
}

#[cfg(not(windows))]
async fn run_command_simple(cmd: &str, tool_name: &str) -> Result<String, DynamicToolError> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return Ok(String::new());
    }

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

#[cfg(not(windows))]
async fn find_linux_desktop_file(app_name: &str) -> Option<String> {
    if cfg!(target_os = "macos") {
        return None;
    }

    let search_paths = vec![
        PathBuf::from("/usr/share/applications"),
        PathBuf::from("/usr/local/share/applications"),
        #[cfg(not(windows))]
        {
            if let Ok(home) = std::env::var("HOME") {
                PathBuf::from(home).join(".local/share/applications")
            } else {
                PathBuf::from("/tmp") // Should not happen on Linux
            }
        },
    ];

    let lower_name = app_name.to_lowercase();

    for path in search_paths {
        if let Ok(mut entries) = tokio::fs::read_dir(&path).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                let file_name = entry.file_name().to_string_lossy().to_lowercase();
                if file_name.ends_with(".desktop") {
                    // Quick check: does the filename contain the app name?
                    if file_name.contains(&lower_name) {
                        return Some(entry.file_name().to_string_lossy().to_string());
                    }

                    // Deep check: read the file to see if the 'Name=' field matches
                    if let Ok(content) = tokio::fs::read_to_string(entry.path()).await {
                        for line in content.lines() {
                            if line.to_lowercase().starts_with("name=") {
                                let name_val = line[5..].trim().to_lowercase();
                                if name_val == lower_name {
                                    return Some(entry.file_name().to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    None
}

pub async fn native_os_tool(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let action = infer_os_tool_action(&input);
    let selector_present = has_uia_selector(&input);

    match action.as_str() {
        "get_state" | "list_windows" => windows_desktop_state_json("os_tool").await,
        "inspect_accessibility" => windows_accessibility_state_json("os_tool", &input).await,
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
            if selector_present {
                return windows_uia_interaction_json("os_tool", "focus_element", &input).await;
            }
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
                if cfg!(target_os = "macos") {
                    let target = extract_os_tool_target(&input, "focus").ok_or_else(|| {
                        DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: "Focus action requires a target window title.".to_string(),
                        }
                    })?;
                    // macOS AppleScript focus
                    let script = format!("tell application \"{}\" to activate", target);
                    let status = Command::new("osascript")
                        .arg("-e")
                        .arg(&script)
                        .status()
                        .await
                        .map_err(|e| DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("Failed to focus {} via AppleScript: {}", target, e),
                        })?;
                    if !status.success() {
                        return Err(DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("osascript failed to focus '{}'.", target),
                        });
                    }
                    return Ok(json!({
                        "action": "focus",
                        "target": target,
                        "success": true,
                        "summary": format!("Focused Mac app '{}' using AppleScript.", target)
                    }).to_string());
                } else {
                    let target = extract_os_tool_target(&input, "focus").ok_or_else(|| {
                        DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: "Focus action requires a target window title.".to_string(),
                        }
                    })?;
                    // Linux focus using wmctrl
                    let status = Command::new("wmctrl")
                        .arg("-a")
                        .arg(&target)
                        .status()
                        .await
                        .map_err(|e| DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("Failed to run wmctrl for {}: {}", target, e),
                        })?;
                    if status.success() {
                        return Ok(json!({
                            "action": "focus",
                            "target": target,
                            "success": true,
                            "summary": format!("Focused window matching '{}' using wmctrl.", target)
                        }).to_string());
                    }
                    return Err(DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("wmctrl could not find or focus a window matching '{}'.", target),
                    });
                }
            }
        }
        "focus_element" => windows_uia_interaction_json("os_tool", "focus_element", &input).await,
        "invoke_element" => {
            windows_uia_interaction_json("os_tool", "invoke_element", &input).await
        }
        "launch" => {
            let app = extract_os_tool_target(&input, "launch").ok_or_else(|| {
                DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Launch action requires an app or command.".to_string(),
                }
            })?;

            #[cfg(windows)]
            {
                let escaped = escape_powershell_single_quoted(&app);
                let script = format!(
                    "Start-Process '{app}'; [pscustomobject]@{{ action='launch'; app='{app}'; ok=$true }} | ConvertTo-Json -Compress",
                    app = escaped
                );
                run_powershell_hidden(&script, "os_tool").await
            }

            #[cfg(not(windows))]
            {
                if cfg!(target_os = "macos") {
                    let status = Command::new("open")
                        .arg("-a")
                        .arg(&app)
                        .status()
                        .await
                        .map_err(|e| DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("Failed to launch Mac app {} using open -a: {}", app, e),
                        })?;
                    if !status.success() {
                        return Err(DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("'open -a' failed to launch Mac app '{}'.", app),
                        });
                    }
                } else {
                    // Linux Smart Launch
                    let mut launched = false;

                    // 1. Try direct command execution if it's potentially a binary path
                    if let Ok(mut child) = Command::new(&app).spawn() {
                        launched = true;
                    }

                    // 2. Try common Linux apps by name (fuzzy) or .desktop files
                    if !launched {
                        if let Some(desktop_file) = find_linux_desktop_file(&app).await {
                             if Command::new("gtk-launch").arg(&desktop_file).spawn().is_ok() {
                                 launched = true;
                             }
                        }
                    }

                    // 3. Fallback to generic open tool
                    if !launched {
                        let _ = Command::new("xdg-open").arg(&app).spawn();
                        launched = true; // Best effort
                    }
                }

                Ok(json!({
                    "action": "launch",
                    "app": app,
                    "success": true,
                    "summary": format!("Submitted launch request for '{}' on {}.", app, if cfg!(target_os = "macos") { "macOS" } else { "Linux" })
                }).to_string())
            }
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

            #[cfg(windows)]
            {
                let escaped = escape_powershell_single_quoted(&url);
                let script = format!(
                    "Start-Process '{url}'; [pscustomobject]@{{ action='open_url'; url='{url}'; ok=$true }} | ConvertTo-Json -Compress",
                    url = escaped
                );
                run_powershell_hidden(&script, "os_tool").await
            }

            #[cfg(not(windows))]
            {
                let cmd = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
                let status = Command::new(cmd)
                    .arg(&url)
                    .status()
                    .await
                    .map_err(|e| DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("Failed to open URL using {}: {}", cmd, e),
                    })?;
                if !status.success() {
                    return Err(DynamicToolError::ExecutionFailed {
                        tool: "os_tool".to_string(),
                        details: format!("{} failed with status {} for URL: {}.", cmd, status, url),
                    });
                }
                Ok(json!({
                    "action": "open_url",
                    "url": url,
                    "success": true,
                    "summary": format!("Opened URL '{}' using {}.", url, cmd)
                }).to_string())
            }
        }
        "click" | "double_click" | "right_click" => {
            if action == "click" && selector_present && extract_coordinates(&input).is_none() {
                return windows_uia_interaction_json("os_tool", "invoke_element", &input).await;
            }
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
        "move_mouse" => {
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
        "type_text" => {
            let text = extract_text_value(&input).ok_or_else(|| {
                DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Type text requires a `text` value.".to_string(),
                }
            })?;
            if selector_present {
                let raw =
                    windows_uia_interaction_json("os_tool", "type_text", &input).await?;
                let mut value: serde_json::Value =
                    serde_json::from_str(&raw).unwrap_or_else(|_| json!({}));
                let method = value
                    .get("method")
                    .and_then(|value| value.as_str())
                    .unwrap_or("");
                if method != "value_pattern" {
                    let mut enigo = with_enigo()?;
                    enigo
                        .text(&text)
                        .map_err(|err| DynamicToolError::ExecutionFailed {
                            tool: "os_tool".to_string(),
                            details: format!("Failed to type text after focusing element: {}", err),
                        })?;
                    if let Some(object) = value.as_object_mut() {
                        object.insert(
                            "method".to_string(),
                            serde_json::Value::String("uia_focus_then_type".to_string()),
                        );
                    }
                }
                if let Some(object) = value.as_object_mut() {
                    object.insert(
                        "typed_chars".to_string(),
                        serde_json::Value::Number((text.chars().count() as u64).into()),
                    );
                    object.insert("ok".to_string(), serde_json::Value::Bool(true));
                }
                return Ok(value.to_string());
            }

            let mut enigo = with_enigo()?;
            enigo
                .text(&text)
                .map_err(|err| DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: format!("Failed to type text: {}", err),
                })?;
            Ok(json!({
                "action": "type_text",
                "typed_chars": text.chars().count(),
                "method": "keyboard_text",
                "ok": true
            })
            .to_string())
        }
        "press_key" => {
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
        "hotkey" => {
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
        "scroll" => {
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
                "Unsupported os_tool action '{}'. Supported actions: get_state, list_windows, inspect_accessibility, browser_state, list_processes, find_files, read_file, open_file, screenshot, focus, focus_element, invoke_element, click, double_click, right_click, move_mouse, type_text, press_key, hotkey, scroll, launch, open_url, wait.",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_os_tool_action_understands_computer_requests() {
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"take a screenshot of my desktop"})),
            "screenshot"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"take a screeshot on my pc"})),
            "screenshot"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"list background tasks"})),
            "list_processes"
        );
        assert_eq!(
            infer_os_tool_action(
                &serde_json::json!({"input":"what is currently running on my computer"})
            ),
            "list_processes"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"inspect accessibility tree"})),
            "inspect_accessibility"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"action":"find_files","query":"resume"})),
            "find_files"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"which browser windows are open"})),
            "browser_state"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"Take a open discord on my pc"})),
            "launch"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"open application Discord"})),
            "launch"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"open https://openai.com"})),
            "open_url"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"focus element Save"})),
            "focus_element"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"invoke Save"})),
            "invoke_element"
        );
        assert_eq!(
            infer_os_tool_action(&serde_json::json!({"input":"click element Save"})),
            "invoke_element"
        );
    }

    #[test]
    fn extract_os_tool_target_prefers_explicit_fields() {
        assert_eq!(
            extract_os_tool_target(&serde_json::json!({"query":"state.rs"}), "find_files"),
            Some("state.rs".to_string())
        );
        assert_eq!(
            extract_os_tool_target(&serde_json::json!({"path":"src/state.rs"}), "read_file"),
            Some("src/state.rs".to_string())
        );
        assert_eq!(
            extract_os_tool_target(&serde_json::json!({"window":"Visual Studio Code"}), "focus"),
            Some("Visual Studio Code".to_string())
        );
        assert_eq!(
            extract_os_tool_target(
                &serde_json::json!({"input":"Take a open discord on my pc"}),
                "launch"
            ),
            Some("discord".to_string())
        );
        assert_eq!(
            extract_os_tool_target(
                &serde_json::json!({"input":"open application Discord"}),
                "launch"
            ),
            Some("Discord".to_string())
        );
        assert_eq!(
            extract_os_tool_target(
                &serde_json::json!({"input":"open website https://openai.com"}),
                "open_url"
            ),
            Some("https://openai.com".to_string())
        );
        assert_eq!(
            build_uia_selector_object(&serde_json::json!({"name":"Save","control_type":"Button"})),
            serde_json::json!({"name":"Save","control_type":"Button"})
        );
        assert_eq!(
            build_uia_selector_object(&serde_json::json!({"input":"click Save button"})),
            serde_json::json!({"name":"Save","control_type":"Button"})
        );
        assert_eq!(
            build_uia_selector_object(
                &serde_json::json!({"input":"type hello@example.com into the Email field"})
            ),
            serde_json::json!({"name":"Email","control_type":"Edit"})
        );
        assert_eq!(
            build_uia_selector_object(&serde_json::json!({"input":"click Save button in Discord"})),
            serde_json::json!({
                "name":"Save",
                "control_type":"Button",
                "app_hint":"discord"
            })
        );
        assert_eq!(
            build_uia_selector_object(
                &serde_json::json!({"input":"focus the Search box in Visual Studio Code"})
            ),
            serde_json::json!({
                "name":"Search",
                "control_type":"Edit",
                "app_hint":"visual studio code"
            })
        );
        assert_eq!(
            build_uia_selector_object(
                &serde_json::json!({"input":"click the Notifications toggle in Discord"})
            ),
            serde_json::json!({
                "name":"Notifications",
                "control_type":"CheckBox",
                "app_hint":"discord"
            })
        );
        assert_eq!(
            build_uia_selector_object(
                &serde_json::json!({"input":"expand the Explorer sidebar section"})
            ),
            serde_json::json!({
                "name":"Explorer",
                "control_type":"TreeItem",
                "app_hint":"explorer"
            })
        );
        assert!(has_uia_selector(&serde_json::json!({
            "action":"click",
            "name":"Save",
            "control_type":"Button"
        })));
        assert!(has_uia_selector(&serde_json::json!({
            "action":"type_text",
            "name":"Email",
            "control_type":"Edit",
            "text":"hello@example.com"
        })));
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "smoke test for live Windows desktop session"]
    async fn native_os_tool_lists_processes() {
        let raw = native_os_tool(&serde_json::json!({"action":"list_processes"}))
            .await
            .expect("list_processes should succeed");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
        assert_eq!(value["action"], "list_processes");
        assert!(
            value
                .get("total_processes")
                .and_then(|v| v.as_i64())
                .unwrap_or(0)
                > 0
        );
        assert!(value
            .get("background_processes")
            .and_then(|v| v.as_array())
            .is_some());
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "smoke test for live Windows desktop session"]
    async fn native_os_tool_finds_repo_files() {
        let raw = native_os_tool(&serde_json::json!({
            "action":"find_files",
            "query":"state.rs"
        }))
        .await
        .expect("find_files should succeed");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
        assert_eq!(value["action"], "find_files");
        let results = value["results"].as_array().expect("results array");
        assert!(results.iter().any(|item| {
            item.get("path")
                .and_then(|v| v.as_str())
                .map(|path| path.to_lowercase().contains("\\overseer\\src\\state.rs"))
                .unwrap_or(false)
        }));
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "smoke test for live Windows desktop session"]
    async fn native_os_tool_reads_repo_file() {
        let path = std::env::current_dir()
            .expect("cwd")
            .join("src")
            .join("state.rs");
        let raw = native_os_tool(&serde_json::json!({
            "action":"read_file",
            "path": path.to_string_lossy().to_string()
        }))
        .await
        .expect("read_file should succeed");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
        assert_eq!(value["action"], "read_file");
        assert!(value["path"]
            .as_str()
            .unwrap_or_default()
            .to_lowercase()
            .contains("state.rs"));
        assert!(value.get("preview").is_some());
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "smoke test for live Windows desktop session"]
    async fn native_os_tool_reports_desktop_state() {
        let raw = native_os_tool(&serde_json::json!({"action":"get_state"}))
            .await
            .expect("get_state should succeed");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
        assert!(value.get("active_window_title").is_some());
        assert!(value
            .get("visible_windows")
            .and_then(|v| v.as_array())
            .is_some());
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "smoke test for live Windows desktop session"]
    async fn native_os_tool_captures_screenshot() {
        let raw = native_os_tool(&serde_json::json!({"action":"screenshot"}))
            .await
            .expect("screenshot should succeed");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
        assert_eq!(value["action"], "screenshot");
        let path = value["path"].as_str().expect("screenshot path");
        assert!(std::path::Path::new(path).exists());
        assert!(value["width"].as_i64().unwrap_or(0) > 0);
        assert!(value["height"].as_i64().unwrap_or(0) > 0);
    }

    #[cfg(windows)]
    #[tokio::test]
    #[ignore = "smoke test for live Windows desktop session"]
    async fn native_os_tool_reports_browser_state() {
        let raw = native_os_tool(&serde_json::json!({"action":"browser_state"}))
            .await
            .expect("browser_state should succeed");
        let value: serde_json::Value = serde_json::from_str(&raw).expect("valid json");
        assert_eq!(value["action"], "browser_state");
        assert!(value
            .get("browser_windows")
            .and_then(|v| v.as_array())
            .is_some());
    }
}
