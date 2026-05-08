use crate::engine::orchestrator::core::result::SupervisorExecutionResult;

pub fn looks_like_screenshot_artifact(v: &str) -> bool {
    let l = v.to_lowercase();
    l.contains(".png") || l.contains("screenshot") || l.contains("\"action\":\"screenshot\"")
}

pub fn looks_like_process_observation(v: &str) -> bool {
    let l = v.to_lowercase();
    if l.contains("\"action\":\"list_processes\"") || l.contains("\"background_processes\"") { return true; }
    parse_json(v).map(|j| {
        j.get("action").and_then(|a| a.as_str()).map(|a| a == "list_processes").unwrap_or(false)
            && (has_arr(&j, "background_processes") || has_arr(&j, "top_processes") || has_count(&j, "total_processes"))
    }).unwrap_or(false)
}

pub fn looks_like_browser_observation(v: &str) -> bool {
    let l = v.to_lowercase();
    if l.contains("\"action\":\"browser_state\"") || l.contains("\"browser_windows\"") { return true; }
    parse_json(v).map(|j| {
        j.get("action").and_then(|a| a.as_str()).map(|a| a == "browser_state").unwrap_or(false)
            && (has_arr(&j, "browser_windows") || j.get("browser_count").is_some())
    }).unwrap_or(false)
}

pub fn looks_like_file_search_observation(v: &str) -> bool {
    let l = v.to_lowercase();
    if l.contains("\"action\":\"find_files\"") || l.contains("\"results\"") { return true; }
    parse_json(v).map(|j| {
        j.get("action").and_then(|a| a.as_str()).map(|a| a == "find_files").unwrap_or(false) && has_arr(&j, "results")
    }).unwrap_or(false)
}

pub fn has_native_evidence(res: &SupervisorExecutionResult, ctx: &str, predicate: impl Fn(&str) -> bool) -> bool {
    res.artifacts.iter().any(|i| predicate(i)) || res.evidence.iter().any(|i| predicate(i)) || predicate(&res.raw_response) || predicate(&res.summary) || predicate(ctx)
}

fn parse_json(v: &str) -> Option<serde_json::Value> { serde_json::from_str(v).ok() }
fn has_arr(j: &serde_json::Value, k: &str) -> bool { j.get(k).and_then(|v| v.as_array()).map(|a| !a.is_empty()).unwrap_or(false) }
fn has_count(j: &serde_json::Value, k: &str) -> bool { j.get(k).and_then(|v| v.as_i64()).map(|c| c > 0).unwrap_or(false) }
