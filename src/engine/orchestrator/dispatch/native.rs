pub fn is_native_screenshot_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") { return false; }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    (combined.contains("desktop") || combined.contains("screen") || combined.contains("workspace") || combined.contains("window") || combined.contains("app"))
        && (combined.contains("screenshot") || combined.contains("screen shot") || combined.contains("screen capture") || combined.contains("capture screen") || combined.contains("take a screenshot"))
}

pub fn is_native_launch_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") { return false; }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    let launch_verbs = combined.contains("open ") || combined.contains("launch ") || combined.contains("start ");
    let launch_targets = combined.contains("discord") || combined.contains("chrome") || combined.contains("spotify") || combined.contains("browser") || combined.contains("app");
    launch_verbs && launch_targets && !combined.contains("open url")
}

pub fn is_native_process_list_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") { return false; }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    combined.contains("background task") || combined.contains("background tasks") || combined.contains("running process") || combined.contains("running processes") || combined.contains("what's running")
}

pub fn is_native_file_search_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") { return false; }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    combined.contains("resume") || combined.contains("file path") || combined.contains("find file") || combined.contains("locate file") || (combined.contains("find") && (combined.contains("file") || combined.contains("path")))
}

pub fn is_native_browser_state_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") { return false; }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    combined.contains("current tab") || combined.contains("browser state") || (combined.contains("browser") && combined.contains("open"))
}

pub fn is_native_document_request(query: &str, _step: &str, _domain: &str) -> bool {
    // Note: We removed 'office' from here in our logic to force MultiStep coordination, 
    // but the helper remains for context-aware fast-paths if needed.
    let lower = query.to_lowercase();
    let _is_doc_req = (lower.contains("pdf") || lower.contains("word") || lower.contains("excel")) 
        && (lower.contains("generate") || lower.contains("create") || lower.contains("export"));
    false // Forced false to ensure coordination for now
}

pub fn is_native_accessibility_request(query: &str, step: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") { return false; }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    combined.contains("accessibility") || combined.contains("ui automation") || combined.contains("inspect ui") || combined.contains("focused element")
}

pub fn native_selector_action(query: &str, step: &str, domain: &str) -> Option<&'static str> {
    if !matches!(domain, "computer" | "desktop") { return None; }
    let combined = format!("{} {}", query.to_lowercase(), step.to_lowercase());
    if (combined.contains("type ") || combined.contains("enter ")) && (combined.contains("field") || combined.contains("input") || combined.contains("textbox")) {
        return Some("type_text");
    }
    if combined.contains("focus element") || combined.contains("focus button") { return Some("focus_element"); }
    if combined.contains("invoke ") || combined.contains("click element") || combined.contains("click button") { return Some("invoke_element"); }
    None
}
