// src/routes/conversations/route_summary.rs
use crate::backend;
use serde_json::Value;

pub fn derive_execution_route_summary(
    events: &[backend::ExecutionEvent],
) -> (Option<String>, Option<String>, Option<String>) {
    let decision = events
        .iter()
        .find(|event| event.event_type == "orchestration_decision");

    let Some(event) = decision else {
        return (None, None, None);
    };

    let details = event
        .details
        .as_deref()
        .and_then(|details| serde_json::from_str::<Value>(details).ok());

    let route = details.as_ref().and_then(|v| v.get("route")).and_then(|v| v.as_str()).map(prettify_route);
    let domain = details.as_ref().and_then(|v| v.get("domain")).and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty() && *v != "null").map(prettify_token);
    let personality = details.as_ref().and_then(|v| v.get("personality_mode")).and_then(|v| v.as_str()).map(prettify_token);
    let reason = details.as_ref().and_then(|v| v.get("reason")).and_then(|v| v.as_str())
        .map(|v| v.to_string()).or_else(|| event.message.clone());

    let route_label = match (route, domain) {
        (Some(route), Some(domain)) if !route.contains(&domain) => Some(format!("{route}: {domain}")),
        (Some(route), _) => Some(route),
        _ => None,
    };

    (route_label, reason, personality)
}

fn prettify_route(value: &str) -> String {
    match value {
        "direct_chat" => "Direct Chat".to_string(),
        "single_domain" => "Single Domain".to_string(),
        "multi_step" => "Multi-Step".to_string(),
        other => prettify_token(other),
    }
}

pub fn prettify_status(value: &str) -> String {
    prettify_token(value)
}

pub fn prettify_token(value: &str) -> String {
    value
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => format!("{}{}", first.to_uppercase(), chars.as_str()),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn status_tone(status: &str) -> &'static str {
    match status {
        "completed" | "success" => "green",
        "failed" | "error" => "red",
        "running" | "in_progress" => "blue",
        _ => "slate",
    }
}
