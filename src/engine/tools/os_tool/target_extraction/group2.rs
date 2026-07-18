pub fn extract_target_group_3(action: &str, lower: &str, explicit: &str) -> Option<String> {
    if action == "click" && lower.starts_with("click ") {
        return Some(explicit[6..].trim().to_string());
    }
    if action == "double_click" && lower.starts_with("double click ") {
        return Some(explicit[13..].trim().to_string());
    }
    if action == "double_click" && lower.starts_with("doubleclick ") {
        return Some(explicit[12..].trim().to_string());
    }
    if action == "right_click" && lower.starts_with("right click ") {
        return Some(explicit[12..].trim().to_string());
    }
    if action == "right_click" && lower.starts_with("rightclick ") {
        return Some(explicit[11..].trim().to_string());
    }
    if action == "move_mouse" && lower.starts_with("move ") {
        return Some(explicit[5..].trim().to_string());
    }
    if action == "move_mouse" && lower.starts_with("hover ") {
        return Some(explicit[6..].trim().to_string());
    }
    if action == "scroll" && lower.starts_with("scroll ") {
        return Some(explicit[7..].trim().to_string());
    }

    if action == "read_file" && lower.starts_with("read ") {
        let name = explicit[5..].trim().to_string();
        if name.contains("file ") {
            return Some(name[5..].trim().to_string());
        }
        return Some(name);
    }
    if action == "open_file" && lower.starts_with("open ") {
        let name = explicit[5..].trim().to_string();
        if name.contains("file ") {
            return Some(name[5..].trim().to_string());
        }
        return Some(name);
    }
    if action == "find_files" && lower.starts_with("find ") {
        let name = explicit[5..].trim().to_string();
        if name.contains("file ") {
            return Some(name[5..].trim().to_string());
        }
        return Some(name);
    }

    None
}
