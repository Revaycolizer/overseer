pub fn infer_action_group_2(lower: &str) -> Option<String> {
    if matches!(
        lower,
        "find_files" | "search_files" | "locate_file"
    ) || (lower.contains("find") && (lower.contains("file") || lower.contains("path")))
        || (lower.contains("locate") && lower.contains("file"))
        || (lower.contains("search") && lower.contains("file"))
    {
        return Some("find_files".to_string());
    }

    if matches!(lower, "read_file" | "read" | "show_file")
        || ((lower.contains("read") || lower.contains("show"))
            && (lower.contains("file")
                || lower.contains(".txt")
                || lower.contains(".md")
                || lower.contains(".json")))
    {
        return Some("read_file".to_string());
    }

    if matches!(lower, "open_file" | "open document" | "open path")
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
        return Some("open_file".to_string());
    }

    if matches!(lower, "browser_state" | "browser" | "tab_state")
        || (lower.contains("browser")
            && (lower.contains("state") || lower.contains("open") || lower.contains("which")))
        || lower.contains("current tab")
        || lower.contains("current page")
    {
        return Some("browser_state".to_string());
    }

    if matches!(lower, "focus_element" | "focus control")
        || lower.starts_with("focus element ")
        || lower.starts_with("focus control ")
    {
        return Some("focus_element".to_string());
    }

    if matches!(lower, "focus" | "activate" | "activate_window")
        || lower.starts_with("focus ")
        || lower.starts_with("activate ")
    {
        return Some("focus".to_string());
    }

    None
}
