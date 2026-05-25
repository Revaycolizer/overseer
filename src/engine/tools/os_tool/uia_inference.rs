pub fn infer_uia_control_type(name: &str) -> Option<&'static str> {
    let lower = name.to_lowercase();
    if lower.contains("button") || lower.contains("btn") || lower.contains("click") {
        Some("button")
    } else if lower.contains("input") || lower.contains("text") || lower.contains("box") || lower.contains("edit") || lower.contains("field") {
        Some("edit")
    } else if lower.contains("checkbox") || lower.contains("chk") {
        Some("checkbox")
    } else if lower.contains("combobox") || lower.contains("combo") || lower.contains("select") || lower.contains("dropdown") {
        Some("combobox")
    } else if lower.contains("link") || lower.contains("hyperlink") {
        Some("hyperlink")
    } else {
        None
    }
}

pub fn clean_uia_selector_name(name: &str) -> String {
    let mut cleaned = name.to_string();
    let prefixes = ["click ", "button ", "btn ", "input ", "text ", "field ", "box ", "link "];
    for prefix in &prefixes {
        if cleaned.to_lowercase().starts_with(prefix) {
            cleaned = cleaned[prefix.len()..].to_string();
            break;
        }
    }
    cleaned.trim().to_string()
}

pub fn infer_uia_app_hint(selector: &str) -> Option<&'static str> {
    infer_uia_app_hint_details(selector).map(|(app, _)| app)
}

pub fn infer_uia_app_hint_details(selector: &str) -> Option<(&'static str, &'static str)> {
    let lower = selector.to_lowercase();
    if lower.contains(" chrome") || lower.contains(" in chrome") || lower.contains(" google chrome") {
        Some(("chrome", "Google Chrome"))
    } else if lower.contains(" edge") || lower.contains(" in edge") || lower.contains(" microsoft edge") {
        Some(("msedge", "Microsoft Edge"))
    } else if lower.contains(" firefox") || lower.contains(" in firefox") || lower.contains(" mozilla firefox") {
        Some(("firefox", "Firefox"))
    } else if lower.contains(" terminal") || lower.contains(" in terminal") || lower.contains(" powershell") || lower.contains(" command prompt") {
        Some(("powershell", "Windows PowerShell"))
    } else if lower.contains(" notepad") || lower.contains(" in notepad") {
        Some(("notepad", "Notepad"))
    } else {
        None
    }
}

pub fn strip_trailing_app_hint(selector: &str) -> String {
    let mut cleaned = selector.to_string();
    let hints = [
        " in chrome", " chrome", " google chrome",
        " in edge", " edge", " microsoft edge",
        " in firefox", " firefox", " mozilla firefox",
        " in terminal", " terminal", " powershell", " command prompt",
        " in notepad", " notepad"
    ];
    for hint in &hints {
        let lower = cleaned.to_lowercase();
        if let Some(pos) = lower.rfind(hint) {
            if pos + hint.len() == lower.len() {
                cleaned = cleaned[..pos].to_string();
            }
        }
    }
    cleaned.trim().to_string()
}
