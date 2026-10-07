use serde_json::Value;

pub fn parse_uia_blocked_payload(payload: &str) -> (Vec<String>, String, String) {
    if let Ok(value) = serde_json::from_str::<Value>(payload) {
        let action = value
            .get("action")
            .and_then(|item| item.as_str())
            .unwrap_or("focus_element");
        let search_scope = value
            .get("search_scope")
            .and_then(|item| item.as_str())
            .unwrap_or("active_window");
        let retry_hint = value
            .get("retry_hint")
            .and_then(|item| item.as_str())
            .unwrap_or("Try a more specific control name.");
        let app_hint = value.get("app_hint").and_then(|item| item.as_str());
        let selector = value.get("selector").and_then(|item| item.as_object());

        let selector_desc = selector
            .map(|s| {
                let mut parts = Vec::new();
                for (k, l) in [
                    ("name", "name"),
                    ("automation_id", "id"),
                    ("class_name", "class"),
                ] {
                    if let Some(v) = s.get(k).and_then(|i| i.as_str()) {
                        if !v.is_empty() {
                            parts.push(format!("{}={}", l, v));
                        }
                    }
                }
                parts.join(", ")
            })
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "the requested selector".to_string());

        let mut evidence = vec![
            format!("Scope: {}.", search_scope),
            format!("Selector: {}.", selector_desc),
        ];
        if let Some(h) = app_hint {
            evidence.push(format!("App window: {}.", h));
        }

        if let Some(candidates) = value.get("candidates").and_then(|i| i.as_array()) {
            for c in candidates.iter().take(3) {
                let name = c.get("name").and_then(|i| i.as_str()).unwrap_or("unnamed");
                let ctype = c
                    .get("control_type")
                    .and_then(|i| i.as_str())
                    .unwrap_or("control");
                evidence.push(format!("Candidate: {} ({})", name, ctype));
            }
        }
        evidence.push(format!("Hint: {}.", retry_hint));

        let summary = match action {
            "invoke_element" => "Blocked: Could not find UI element to invoke.".to_string(),
            _ => "Blocked: Could not find UI element to focus.".to_string(),
        };
        return (
            evidence,
            summary,
            format!("No UI element matched. {}", retry_hint),
        );
    }
    (
        vec!["No UI element found.".to_string()],
        "Blocked.".to_string(),
        "No UI element found.".to_string(),
    )
}

pub fn extract_uia_blocked_json(error: &str) -> Option<&str> {
    error
        .find("UIA_BLOCKED:")
        .map(|index| error[index + "UIA_BLOCKED:".len()..].trim())
}
