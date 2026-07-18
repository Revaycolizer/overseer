pub fn extract_target_group_1(action: &str, lower: &str, explicit: &str) -> Option<String> {
    if action == "type_text" {
        if lower.starts_with("type ") {
            return Some(explicit[5..].trim().to_string());
        }
        if lower.starts_with("write ") {
            return Some(explicit[6..].trim().to_string());
        }
        if lower.starts_with("input text ") {
            return Some(explicit[11..].trim().to_string());
        }
    }

    if action == "press_key" {
        if lower.starts_with("press ") {
            return Some(explicit[6..].trim().to_string());
        }
        if lower.starts_with("key ") {
            return Some(explicit[4..].trim().to_string());
        }
    }

    if action == "hotkey" {
        if lower.starts_with("hotkey ") {
            return Some(explicit[7..].trim().to_string());
        }
        if lower.starts_with("shortcut ") {
            return Some(explicit[9..].trim().to_string());
        }
        if lower.starts_with("combination ") {
            return Some(explicit[12..].trim().to_string());
        }
    }

    if action == "wait" {
        if lower.starts_with("wait ") {
            return Some(explicit[5..].trim().to_string());
        }
        if lower.starts_with("sleep ") {
            return Some(explicit[6..].trim().to_string());
        }
        if lower.starts_with("delay ") {
            return Some(explicit[6..].trim().to_string());
        }
    }

    None
}

pub fn extract_target_group_2(action: &str, lower: &str, explicit: &str) -> Option<String> {
    if action == "launch" {
        if lower.starts_with("launch ") {
            return Some(explicit[7..].trim().to_string());
        }
        if lower.starts_with("run ") {
            return Some(explicit[4..].trim().to_string());
        }
        if lower.starts_with("execute ") {
            return Some(explicit[8..].trim().to_string());
        }
    }

    if action == "open_url" {
        if lower.starts_with("open ") {
            return Some(explicit[5..].trim().to_string());
        }
        if lower.starts_with("visit ") {
            return Some(explicit[6..].trim().to_string());
        }
        if lower.starts_with("go to ") {
            return Some(explicit[6..].trim().to_string());
        }
    }

    if action == "focus" {
        if lower.starts_with("focus ") {
            return Some(explicit[6..].trim().to_string());
        }
        if lower.starts_with("activate ") {
            return Some(explicit[9..].trim().to_string());
        }
    }

    None
}
