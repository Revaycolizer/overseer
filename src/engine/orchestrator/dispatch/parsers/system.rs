use serde_json::Value;

pub fn parse_browser_state_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let mut evidence = Vec::new();
        if let Some(windows) = value.get("browser_windows").and_then(|item| item.as_array()) {
            for window in windows.iter().take(12) {
                let name = window.get("ProcessName").and_then(|item| item.as_str()).unwrap_or("browser");
                let title = window.get("MainWindowTitle").and_then(|item| item.as_str()).unwrap_or("Untitled");
                evidence.push(format!("{name}: {title}"));
            }
        }
        let summary = if evidence.is_empty() { "No open browser windows were detected.".to_string() } 
        else { "Observed the current open browser windows on this machine.".to_string() };
        return (evidence, summary);
    }
    (vec!["Browser windows were inspected natively.".to_string()], "Observed the current browser state.".to_string())
}

pub fn parse_file_search_payload(payload: &str) -> (Vec<String>, Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let mut evidence = Vec::new();
        let mut artifacts = Vec::new();
        if let Some(results) = value.get("results").and_then(|item| item.as_array()) {
            for result in results.iter().take(12) {
                let name = result.get("name").and_then(|item| item.as_str()).unwrap_or("unknown");
                let path = result.get("path").and_then(|item| item.as_str()).unwrap_or("");
                if !path.is_empty() {
                    evidence.push(format!("{name}: {path}"));
                    artifacts.push(path.to_string());
                }
            }
        }
        let summary = if let Some(first) = artifacts.first() { format!("Found matching file paths on this PC. Best match: {}", first) } 
        else { "No matching file path was found on this PC.".to_string() };
        return (evidence, artifacts, summary);
    }
    (vec!["Local file search completed.".to_string()], Vec::new(), "Local file search completed.".to_string())
}

pub fn parse_process_list_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let total = value.get("total_processes").and_then(|item| item.as_i64()).unwrap_or(0);
        let mut evidence = Vec::new();
        if let Some(processes) = value.get("background_processes").and_then(|item| item.as_array()) {
            for process in processes.iter().take(12) {
                let name = process.get("ProcessName").and_then(|item| item.as_str()).unwrap_or("unknown");
                let pid = process.get("Id").and_then(|item| item.as_i64()).unwrap_or_default();
                let cpu = process.get("CPU").and_then(|item| item.as_f64()).unwrap_or(0.0);
                evidence.push(format!("{name} (PID {pid}, CPU {cpu:.1})"));
            }
        }
        let summary = if total > 0 { format!("Observed {} running processes. Here are the current background tasks with the highest activity.", total) } 
        else { "Observed the current running background tasks.".to_string() };
        return (evidence, summary);
    }
    (vec!["Current running processes were inspected natively.".to_string()], "Observed the current running background tasks.".to_string())
}

pub fn parse_screenshot_artifact_payload(payload: &str) -> (Vec<String>, Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let path = value.get("path").and_then(|item| item.as_str()).unwrap_or("screenshot artifact");
        let width = value.get("width").and_then(|item| item.as_i64());
        let height = value.get("height").and_then(|item| item.as_i64());
        let summary = match (width, height) {
            (Some(w), Some(h)) => format!("Captured a desktop screenshot at {}x{}.", w, h),
            _ => "Captured a desktop screenshot.".to_string(),
        };
        let mut evidence = vec![format!("Native screenshot artifact created at {}.", path)];
        if let (Some(w), Some(h)) = (width, height) { evidence.push(format!("Screenshot dimensions: {}x{}.", w, h)); }
        return (evidence, vec![path.to_string()], summary);
    }
    (vec!["Native screenshot artifact created.".to_string()], vec!["screenshot".to_string()], "Captured a desktop screenshot.".to_string())
}

pub fn parse_launch_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let app = value.get("app").and_then(|item| item.as_str()).unwrap_or("the requested app");
        return (vec![format!("Native app launch executed for {}.", app)], format!("Launched {}.", app));
    }
    (vec!["Native app launch executed.".to_string()], "Launched the requested app.".to_string())
}

pub fn parse_document_writer_payload(payload: &str) -> (Vec<String>, Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let mut evidence = Vec::new();
        let mut artifacts = Vec::new();
        let message = value.get("message").and_then(|m| m.as_str()).unwrap_or("Document generation completed.");
        if let Some(items) = value.get("artifacts").and_then(|a| a.as_array()) {
            for item in items {
                let path = item.get("path").and_then(|p| p.as_str()).unwrap_or("");
                let label = item.get("label").and_then(|l| l.as_str()).unwrap_or("Export");
                if !path.is_empty() {
                    evidence.push(format!("{}: {}", label, path));
                    artifacts.push(path.to_string());
                }
            }
        }
        return (evidence, artifacts, message.to_string());
    }
    (vec!["Document generation completed.".to_string()], Vec::new(), "Document generation completed.".to_string())
}
