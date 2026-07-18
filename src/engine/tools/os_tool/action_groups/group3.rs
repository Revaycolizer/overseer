pub fn infer_action_group_3(lower: &str) -> Option<String> {
    if matches!(
        lower,
        "screenshot" | "capture" | "snap" | "print_screen" | "printscreen"
    ) || lower.contains("take a screenshot")
        || lower.contains("take screenshot")
        || lower.contains("capture the screen")
        || lower.contains("capture screen")
        || lower.contains("snap screen")
    {
        return Some("screenshot".to_string());
    }

    if lower.starts_with("double click") || lower.starts_with("doubleclick") {
        return Some("double_click".to_string());
    }
    if lower.starts_with("right click") || lower.starts_with("rightclick") {
        return Some("right_click".to_string());
    }
    if lower.starts_with("invoke") || lower.starts_with("execute element") {
        return Some("invoke_element".to_string());
    }
    if lower.starts_with("click") {
        return Some("click".to_string());
    }
    if lower.starts_with("move") || lower.starts_with("hover") {
        return Some("move_mouse".to_string());
    }
    if lower.starts_with("type ") || lower.starts_with("write ") || lower.starts_with("input text ") {
        return Some("type_text".to_string());
    }
    if lower.starts_with("press ") || lower.starts_with("key ") {
        return Some("press_key".to_string());
    }
    if lower.starts_with("hotkey ") || lower.starts_with("shortcut ") || lower.starts_with("combination ") {
        return Some("hotkey".to_string());
    }
    if lower.starts_with("scroll ") {
        return Some("scroll".to_string());
    }

    if matches!(lower, "launch" | "run" | "execute")
        || lower.starts_with("launch ")
        || lower.starts_with("run ")
        || lower.starts_with("execute ")
    {
        return Some("launch".to_string());
    }

    if matches!(
        lower,
        "open_url" | "open link" | "open web page" | "open webpage"
    ) || lower.starts_with("open ")
        || lower.starts_with("open_url ")
        || lower.starts_with("visit ")
        || lower.starts_with("go to ")
        || lower.contains("http://")
        || lower.contains("https://")
    {
        return Some("open_url".to_string());
    }

    if lower.starts_with("wait ") || lower.starts_with("sleep ") || lower.starts_with("delay ") {
        return Some("wait".to_string());
    }

    None
}
