pub mod discovery;

use tokio::process::Command;
use crate::engine::tools::{BrowserTool, BrowserArgs};
use rig::tool::Tool;

pub async fn get_browser_dom_tree() -> Option<String> {
    let tool = BrowserTool::new();
    let args = BrowserArgs {
        action: "get_dom_tree".to_string(),
        url: None,
        text: None,
        selector: None,
        submit: None,
        direction: None,
        amount: None,
        navigate: None,
        x: None,
        y: None,
    };
    
    match tool.call(args).await {
        Ok(out) => {
            let mut json_str = "";
            for line in out.lines() {
                if line.trim().starts_with('{') {
                    json_str = line.trim();
                    break;
                }
            }
            if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(dom_tree) = json_val.get("dom_tree").and_then(|v| v.as_str()) {
                    return Some(dom_tree.to_string());
                }
            }
            None
        },
        Err(_) => None,
    }
}

pub async fn focus_window_by_pid(pid: i32) {
    println!("🎯 TARGET-FOCUS: Attempting to focus window for PID {}...", pid);
    // wmctrl -lp returns: <window_id> <desktop> <pid> <machine> <title>
    let output = Command::new("wmctrl")
        .arg("-lp")
        .output()
        .await;

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[2] == pid.to_string() {
                let xid = parts[0];
                println!("🎯 TARGET-FOCUS: Found XID {} for PID {}. Focusing...", xid, pid);
                let _ = Command::new("wmctrl")
                    .arg("-i")
                    .arg("-a")
                    .arg(xid)
                    .output()
                    .await;
                return;
            }
        }
    }
    
    // Fallback if PID search fails
    let _ = Command::new("wmctrl").arg("-x").arg("-a").arg("chromium").output().await;
    let _ = Command::new("wmctrl").arg("-x").arg("-a").arg("google-chrome").output().await;
}

pub fn clean_response(response: String) -> String {
    let mut s = response;
    
    // Remove think blocks
    while let Some(start) = s.find("<think>") {
        if let Some(end) = s.find("</think>") {
            s.replace_range(start..=(end + "</think>".len() - 1), "");
        } else {
            s.truncate(start); break;
        }
    }

    // Extract JSON from Markdown code blocks if present
    if let Some(start) = s.find("```json") {
        let after_json = &s[start + 7..];
        if let Some(end) = after_json.find("```") {
            return after_json[..end].trim().to_string();
        }
    } else if let Some(start) = s.find("```") {
        let after_block = &s[start + 3..];
        if let Some(end) = after_block.find("```") {
            let block = after_block[..end].trim();
            if block.starts_with('{') || block.starts_with('[') {
                return block.to_string();
            }
        }
    }

    s.trim().to_string()
}
