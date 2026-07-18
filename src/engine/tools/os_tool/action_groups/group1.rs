pub fn infer_action_group_1(lower: &str) -> Option<String> {
    if matches!(
        lower,
        "get_state" | "state" | "observe" | "inspect" | "snapshot" | "status"
    ) || lower.contains("current state")
        || lower.contains("observe desktop")
        || lower.contains("screen state")
    {
        return Some("get_state".to_string());
    }

    if matches!(lower, "list_windows" | "windows" | "list")
        || (lower.contains("list") && lower.contains("window"))
    {
        return Some("list_windows".to_string());
    }

    if matches!(
        lower,
        "inspect_accessibility"
            | "accessibility"
            | "accessibility_tree"
            | "ui_tree"
            | "focused_element"
            | "uia"
    ) || lower.contains("accessibility tree")
        || lower.contains("ui automation")
        || lower.contains("automation tree")
        || lower.contains("focused element")
        || lower.contains("focused control")
        || lower.contains("inspect accessibility")
        || lower.contains("inspect ui")
    {
        return Some("inspect_accessibility".to_string());
    }

    if matches!(
        lower,
        "list_processes" | "processes" | "tasks" | "background_tasks" | "background processes"
    ) || ((lower.contains("background") || lower.contains("running"))
        && (lower.contains("task") || lower.contains("process")))
        || lower.contains("what's running")
        || lower.contains("whats running")
        || ((lower.contains("currently running")
            || lower.contains("what is running")
            || lower.contains("what is currently running"))
            && (lower.contains("computer")
                || lower.contains("pc")
                || lower.contains("machine")
                || lower.contains("system")))
    {
        return Some("list_processes".to_string());
    }

    None
}
