use regex;
use serde::{Deserialize, Serialize};
use serde_json;

#[derive(Deserialize, Serialize, Clone)]
pub struct MacroTask {
    pub domain: String,
    pub task: String,
}

pub fn sanitize_plan(plan_text: &str, query: &str) -> Vec<MacroTask> {
    // Extract JSON array
    let clean_text = plan_text.replace("\\\"", "\"").replace('\n', " ");
    let start = clean_text.find('[');
    let end = clean_text.rfind(']');
    let json_candidate = if let (Some(s), Some(e)) = (start, end) {
        if s < e {
            clean_text[s..e + 1].to_string()
        } else {
            clean_text.clone()
        }
    } else {
        clean_text.clone()
    };

    let macro_tasks: Vec<MacroTask> = match serde_json::from_str(&json_candidate) {
        Ok(tasks) => tasks,
        Err(e) => {
            // Default fallback if JSON fails
            tracing::warn!(
                ?e,
                "Failed to parse master planner JSON, routing to chat fallback"
            );
            vec![MacroTask {
                domain: "chat".to_string(),
                task: query.to_string(),
            }]
        }
    };

    macro_tasks
}

pub fn parse_micro_steps(plan_text: &str, fallback_query: &str) -> Vec<String> {
    let clean_text = plan_text.replace("\\\"", "\"").replace('\n', " ");
    let s = clean_text.find('[');
    let e = clean_text.rfind(']');
    let json_candidate = if let (Some(s), Some(e)) = (s, e) {
        if s < e {
            clean_text[s..e + 1].to_string()
        } else {
            clean_text.clone()
        }
    } else {
        clean_text.clone()
    };

    match serde_json::from_str::<serde_json::Value>(&json_candidate) {
        Ok(serde_json::Value::Array(arr)) => {
            let parsed: Vec<String> = arr
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if parsed.is_empty() {
                vec![fallback_query.to_string()]
            } else {
                parsed
            }
        }
        _ => {
            let s_re = regex::Regex::new(r#""([^"\\]*(?:\\.[^"\\]*)*)""#).unwrap();
            let scavenged: Vec<String> = s_re
                .captures_iter(&json_candidate)
                .map(|cap| cap[1].replace("\\\"", "\""))
                .filter(|s| s.len() > 3 && !s.contains('{') && !s.contains('}'))
                .collect();
            if scavenged.is_empty() {
                vec![fallback_query.to_string()]
            } else {
                scavenged
            }
        }
    }
}
