use serde_json::Value;
use crate::engine::orchestrator::state::StructuredState;
use regex::Regex;

pub fn expand_markers(mut value: Value, state: &StructuredState) -> Value {
    match value {
        Value::String(s) => Value::String(expand_string_markers(s, state)),
        Value::Array(mut arr) => {
            for item in arr.iter_mut() {
                *item = expand_markers(item.clone(), state);
            }
            Value::Array(arr)
        },
        Value::Object(mut obj) => {
            for (_, val) in obj.iter_mut() {
                *val = expand_markers(val.clone(), state);
            }
            Value::Object(obj)
        },
        _ => value,
    }
}

fn expand_string_markers(text: String, state: &StructuredState) -> String {
    let re = Regex::new(r"\[E(\d+)\]").unwrap();
    let mut expanded = text;
    
    // Reverse to avoid offset issues
    let matches: Vec<_> = re.find_iter(&expanded).map(|m| m.as_str().to_string()).collect();
    for m in matches {
        if let Some(id_str) = m.strip_prefix("[E").and_then(|s| s.strip_suffix("]")) {
            let full_id = format!("E{}", id_str);
            if let Some(chunk) = state.evidence.iter().find(|e| e.id == full_id) {
                let replacement = format!("\"{}\" [Grounded ID: {}]", chunk.text, chunk.snowflake_id);
                expanded = expanded.replace(&m, &replacement);
            }
        }
    }
    expanded
}
