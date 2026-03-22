pub mod discovery;

use rig::tool::Tool;
use tokio::process::Command;

pub async fn get_browser_dom_tree() -> Option<String> {
    // Note: Legacy static tool call removed.
    // Domain-specific supervisors now use dynamic tools directly.
    None
}

pub async fn focus_window_by_pid(pid: i32) {
    println!(
        "🎯 TARGET-FOCUS: Attempting to focus window for PID {}...",
        pid
    );
    // wmctrl -lp returns: <window_id> <desktop> <pid> <machine> <title>
    let output = Command::new("wmctrl").arg("-lp").output().await;

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[2] == pid.to_string() {
                let xid = parts[0];
                println!(
                    "🎯 TARGET-FOCUS: Found XID {} for PID {}. Focusing...",
                    xid, pid
                );
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
    let _ = Command::new("wmctrl")
        .arg("-x")
        .arg("-a")
        .arg("chromium")
        .output()
        .await;
    let _ = Command::new("wmctrl")
        .arg("-x")
        .arg("-a")
        .arg("google-chrome")
        .output()
        .await;
}

pub fn clean_response(response: String) -> String {
    let mut s = response;

    // Remove think blocks
    while let Some(start) = s.find("<think>") {
        if let Some(end) = s.find("</think>") {
            s.replace_range(start..=(end + "</think>".len() - 1), "");
        } else {
            s.truncate(start);
            break;
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

    // Fallback: If it's a raw JSON object but has trailing text, find the matching brace
    let trimmed = s.trim();
    if trimmed.starts_with('{') {
        let mut brace_count = 0;
        let mut end_index = 0;
        let mut in_string = false;
        let mut escaped = false;

        for (i, c) in trimmed.chars().enumerate() {
            if c == '"' && !escaped {
                in_string = !in_string;
            }
            if !in_string {
                if c == '{' {
                    brace_count += 1;
                } else if c == '}' {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end_index = i + 1;
                        break;
                    }
                }
            }
            escaped = !escaped && c == '\\';
        }

        if end_index > 0 {
            return trimmed[..end_index].to_string();
        }
    }

    s.trim().to_string()
}
