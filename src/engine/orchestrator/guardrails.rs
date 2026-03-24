use crate::engine::utils::discovery::ToolInfo;

pub struct GuardrailResult {
    pub allowed: bool,
    pub reason: Option<String>,
}

pub fn check_tool_guardrails(
    query: &str,
    step: &str,
    tool: &ToolInfo,
    input: &serde_json::Value,
) -> GuardrailResult {
    // 1. Safety Level Check
    let safety_level = tool.safety_level.to_lowercase();
    let is_high_risk = matches!(safety_level.as_str(), "high" | "dangerous");

    // 2. Intent Matching Check
    let required_intent = tool.required_intent.as_deref().unwrap_or("");
    let keywords: Vec<&str> = required_intent
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let user_query_lower = query.to_lowercase();
    let user_intent_matches = if keywords.is_empty() {
        true // No specific intent required
    } else {
        keywords.iter().any(|&k| user_query_lower.contains(k))
    };

    // 3. Enforcement Logic
    if is_high_risk && !user_intent_matches {
        return GuardrailResult {
            allowed: false,
            reason: Some(format!(
                "Tool '{}' is marked as '{}' risk and requires explicit user intent matching keywords: [{}].",
                tool.name, safety_level, required_intent
            )),
        };
    }

    if !user_intent_matches && !required_intent.is_empty() {
        return GuardrailResult {
            allowed: false,
            reason: Some(format!(
                "Tool '{}' requires specific intent keywords [{}], but the current request does not match.",
                tool.name, required_intent
            )),
        };
    }

    GuardrailResult {
        allowed: true,
        reason: None,
    }
}
