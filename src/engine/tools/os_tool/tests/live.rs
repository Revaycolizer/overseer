//! Live desktop-session smoke tests (Windows).
//!
//! These exercise the real OS: process list, file search/read, window/browser
//! state, and screenshots. They are compiled only with the `live-hardware-tests`
//! feature (see `tests/mod.rs`) and only on Windows, so the default `cargo test`
//! never touches a display or input device. Every action is wrapped in a hard
//! timeout so a stalled session fails the test instead of hanging the suite.

#![cfg(windows)]

use crate::engine::tools::os_tool::native_os_tool;
use std::time::Duration;

/// Run an `os_tool` action with a hard timeout and parse the JSON result.
async fn action(value: serde_json::Value) -> serde_json::Value {
    let raw = tokio::time::timeout(Duration::from_secs(30), native_os_tool(&value))
        .await
        .expect("os_tool action timed out (no live desktop session?)")
        .expect("os_tool action should succeed");
    serde_json::from_str(&raw).expect("valid json")
}

#[tokio::test]
async fn native_os_tool_lists_processes() {
    let value = action(serde_json::json!({ "action": "list_processes" })).await;
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

#[tokio::test]
async fn native_os_tool_finds_repo_files() {
    let value = action(serde_json::json!({ "action": "find_files", "query": "state.rs" })).await;
    assert_eq!(value["action"], "find_files");
    let results = value["results"].as_array().expect("results array");
    assert!(results.iter().any(|item| {
        item.get("path")
            .and_then(|v| v.as_str())
            .map(|path| path.to_lowercase().contains("\\overseer\\src\\state.rs"))
            .unwrap_or(false)
    }));
}

#[tokio::test]
async fn native_os_tool_reads_repo_file() {
    let path = std::env::current_dir()
        .expect("cwd")
        .join("src")
        .join("state.rs");
    let value = action(serde_json::json!({
        "action": "read_file",
        "path": path.to_string_lossy().to_string()
    }))
    .await;
    assert_eq!(value["action"], "read_file");
    assert!(value["path"]
        .as_str()
        .unwrap_or_default()
        .to_lowercase()
        .contains("state.rs"));
    assert!(value.get("preview").is_some());
}

#[tokio::test]
async fn native_os_tool_reports_desktop_state() {
    let value = action(serde_json::json!({ "action": "get_state" })).await;
    assert!(value.get("active_window_title").is_some());
    assert!(value
        .get("visible_windows")
        .and_then(|v| v.as_array())
        .is_some());
}

#[tokio::test]
async fn native_os_tool_captures_screenshot() {
    let value = action(serde_json::json!({ "action": "screenshot" })).await;
    assert_eq!(value["action"], "screenshot");
    let path = value["path"].as_str().expect("screenshot path");
    assert!(std::path::Path::new(path).exists());
    assert!(value["width"].as_i64().unwrap_or(0) > 0);
    assert!(value["height"].as_i64().unwrap_or(0) > 0);
}

#[tokio::test]
async fn native_os_tool_reports_browser_state() {
    let value = action(serde_json::json!({ "action": "browser_state" })).await;
    assert_eq!(value["action"], "browser_state");
    assert!(value
        .get("browser_windows")
        .and_then(|v| v.as_array())
        .is_some());
}
