pub mod discovery;
pub mod system_idle;

use tokio::process::Command;

pub async fn get_browser_dom_tree() -> Option<String> {
    // Note: Legacy static tool call removed.
    // Domain-specific supervisors now use dynamic tools directly.
    None
}

pub async fn focus_window_by_pid(pid: i32) {
    tracing::debug!(?pid, "TARGET-FOCUS: attempting to focus window");
    // wmctrl -lp returns: <window_id> <desktop> <pid> <machine> <title>
    let output = Command::new("wmctrl").arg("-lp").output().await;

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[2] == pid.to_string() {
                let xid = parts[0];
                tracing::debug!(?xid, ?pid, "TARGET-FOCUS: found XID, focusing");
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
    let json_content = if let Some(start) = s.find("```json") {
        let after_json = &s[start + 7..];
        if let Some(end) = after_json.find("```") {
            after_json[..end].trim().to_string()
        } else {
            after_json.trim().to_string() // Truncated but started with ```json
        }
    } else if let Some(start) = s.find("```") {
        let after_block = &s[start + 3..];
        if let Some(end) = after_block.find("```") {
            let block = after_block[..end].trim();
            if block.starts_with('{') || block.starts_with('[') {
                block.to_string()
            } else {
                s.trim().to_string()
            }
        } else {
            let block = after_block.trim();
            if block.starts_with('{') || block.starts_with('[') {
                block.to_string()
            } else {
                s.trim().to_string()
            }
        }
    } else {
        s.trim().to_string()
    };

    // If it's a raw JSON object (or truncated one), find the start and attempt to repair
    if let Some(start_idx) = json_content.find('{') {
        let candidate = &json_content[start_idx..];

        // If it parses directly, return it
        if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
            return candidate.to_string();
        }

        // Attempt repair for truncation
        return repair_json_truncation(candidate);
    }

    json_content
}

fn repair_json_truncation(input: &str) -> String {
    let mut repaired = input.to_string();
    let mut brace_stack = Vec::new();
    let mut in_string = false;
    let mut escaped = false;

    for c in input.chars() {
        if c == '"' && !escaped {
            in_string = !in_string;
        }
        if !in_string {
            if c == '{' {
                brace_stack.push('}');
            } else if c == '[' {
                brace_stack.push(']');
            } else if c == '}' || c == ']' {
                if let Some(&expected) = brace_stack.last() {
                    if expected == c {
                        brace_stack.pop();
                    }
                }
            }
        }
        escaped = !escaped && c == '\\';
    }

    // If we are left inside a string, close it
    if in_string {
        // If it looks like it was cut off in the middle of a word or bool
        // e.g. "verified":tru -> close the quote first
        repaired.push('"');
    }

    // Close any dangling braces/brackets in reverse order
    while let Some(closing) = brace_stack.pop() {
        repaired.push(closing);
    }

    // Final check: Is it valid now?
    if serde_json::from_str::<serde_json::Value>(&repaired).is_ok() {
        return repaired;
    }

    // One more try: If the last char is a comma, it's definitely truncated mid-field
    let trimmed = repaired.trim();
    if trimmed.ends_with(',') {
        let final_repaired = trimmed[..trimmed.len() - 1].to_string();
        // Recalculate braces for the shorter version
        // (Simplified: just re-run the same logic)
        return repair_json_truncation(&final_repaired);
    }

    // Fallback to original input if repair failed to produce valid JSON
    input.to_string()
}
