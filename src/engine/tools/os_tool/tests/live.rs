use crate::engine::tools::os_tool::native_os_tool;

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
    assert_eq!(value["status"], "success", "screenshot should report success: {}", raw);
    let path = value["path"].as_str().expect("screenshot path");
    assert!(std::path::Path::new(path).exists(), "screenshot file should exist at {}", path);
    assert!(value["mime"] == "image/png", "mime should be image/png");
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
