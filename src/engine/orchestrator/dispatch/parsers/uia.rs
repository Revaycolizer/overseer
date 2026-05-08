use serde_json::Value;

pub fn parse_accessibility_payload(payload: &str) -> (Vec<String>, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let mut evidence = Vec::new();
        let scope = value.get("search_scope").and_then(|item| item.as_str()).unwrap_or("active_window");
        let match_count = value.get("match_count").and_then(|item| item.as_u64()).unwrap_or(0);

        if let Some(focused) = value.get("focused_element").and_then(|item| item.as_object()) {
            let name = focused.get("name").and_then(|item| item.as_str()).unwrap_or("unnamed element");
            let control_type = focused.get("control_type").and_then(|item| item.as_str()).unwrap_or("control");
            evidence.push(format!("Focused element: {} ({})", name, control_type));
        }

        if let Some(matches) = value.get("matches").and_then(|item| item.as_array()) {
            for matched in matches.iter().take(5) {
                let name = matched.get("name").and_then(|item| item.as_str()).unwrap_or("unnamed element");
                let control_type = matched.get("control_type").and_then(|item| item.as_str()).unwrap_or("control");
                evidence.push(format!("Matched element: {} ({})", name, control_type));
            }
        }
        evidence.push(format!("Accessibility search scope: {}.", scope));
        let summary = if match_count > 0 {
            format!("Inspected accessibility state and found {} matching UI elements.", match_count)
        } else { "Inspected accessibility state for the current UI scope.".to_string() };
        return (evidence, summary);
    }
    (vec!["Accessibility state was inspected natively.".to_string()], "Inspected accessibility state for the current UI scope.".to_string())
}

pub fn parse_uia_interaction_payload(payload: &str) -> (Vec<String>, String, bool) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let action = value.get("action").and_then(|item| item.as_str()).unwrap_or("focus_element");
        let method = value.get("method").and_then(|item| item.as_str()).unwrap_or("uia");
        let scope = value.get("search_scope").and_then(|item| item.as_str()).unwrap_or("active_window");
        let element = value.get("element").and_then(|item| item.as_object());
        let verified = value.get("verified").and_then(|item| item.as_bool()).unwrap_or(false);
        let verification = value.get("verification").and_then(|item| item.as_str()).unwrap_or("not_checked");
        let name = element.and_then(|item| item.get("name")).and_then(|item| item.as_str()).unwrap_or("target element");
        let control_type = element.and_then(|item| item.get("control_type")).and_then(|item| item.as_str()).unwrap_or("control");

        let mut evidence = vec![
            format!("Matched UI element: {} ({})", name, control_type),
            format!("Interaction method: {}.", method),
            format!("UIA search scope: {}.", scope),
            format!("Verification: {}.", verification.replace('_', " ")),
        ];
        if let Some(text_length) = value.get("text_length").and_then(|item| item.as_u64()) {
            evidence.push(format!("Typed {} characters through the UI control.", text_length));
        }
        if let Some(post_focus) = value.get("post_action_focused_element").and_then(|item| item.as_object()) {
            let focus_name = post_focus.get("name").and_then(|item| item.as_str()).unwrap_or("unnamed element");
            let focus_type = post_focus.get("control_type").and_then(|item| item.as_str()).unwrap_or("control");
            evidence.push(format!("Focused after action: {} ({})", focus_name, focus_type));
        }

        let summary = match action {
            "invoke_element" => format!("Invoked UI element '{}'.", name),
            "type_text" => format!("Entered text into UI element '{}'.", name),
            _ => format!("Focused UI element '{}'.", name),
        };
        return (evidence, summary, verified);
    }
    (vec!["A UI Automation element interaction completed natively.".to_string()], "Completed a native UI Automation interaction.".to_string(), false)
}

