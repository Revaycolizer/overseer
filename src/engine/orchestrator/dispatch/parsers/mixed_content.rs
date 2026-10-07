use crate::engine::orchestrator::dispatch::parsers::json_repair::repair_json_literals;
use serde_json::Value;

pub struct ParseResult {
    pub merged_value: serde_json::Map<String, Value>,
    pub summary: String,
    pub json_found: bool,
}

pub fn parse_mixed_content(raw: &str) -> ParseResult {
    let mut merged_value = serde_json::Map::new();
    let mut summary = String::new();
    let mut json_found = false;
    let mut brace_count = 0;
    let mut json_start = None;
    let chars: Vec<char> = raw.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if c == '{' {
            if brace_count == 0 {
                json_start = Some(i);
            }
            brace_count += 1;
        } else if c == '}' {
            if brace_count > 0 {
                brace_count -= 1;
                if brace_count == 0 {
                    if let Some(start) = json_start {
                        let potential_raw: String = chars[start..=i].iter().collect();
                        let potential = repair_json_literals(&potential_raw);
                        if let Ok(val) = serde_json::from_str::<Value>(&potential) {
                            json_found = true;
                            merge_json_into_state(&mut merged_value, &mut summary, val);
                        }
                    }
                }
            }
        } else if brace_count == 0 {
            summary.push(c);
        }
    }
    ParseResult {
        merged_value,
        summary,
        json_found,
    }
}

fn merge_json_into_state(
    merged_value: &mut serde_json::Map<String, Value>,
    summary: &mut String,
    val: Value,
) {
    let target = val.get("final").unwrap_or(&val);
    if let Some(obj) = target.as_object() {
        if obj.contains_key("chart_type")
            && (obj.contains_key("labels") || obj.contains_key("data"))
        {
            let arr = merged_value
                .entry("charts".to_string())
                .or_insert(Value::Array(Vec::new()))
                .as_array_mut()
                .unwrap();
            arr.push(target.clone());
        }
        for (k, v) in obj {
            match k.as_str() {
                "charts" | "evidence" | "evidences" | "artifacts" | "attachments" | "messages" => {
                    let entry = merged_value
                        .entry(k.clone())
                        .or_insert(Value::Array(Vec::new()));
                    if let Some(arr) = entry.as_array_mut() {
                        if let Some(new_arr) = v.as_array() {
                            arr.extend(new_arr.clone());
                        } else {
                            arr.push(v.clone());
                        }
                    }
                }
                "status" | "verified" => {
                    merged_value.insert(k.clone(), v.clone());
                }
                "summary" | "message" => {
                    if let Some(s) = extract_text_from_value(v) {
                        let trimmed = s.trim();
                        if !trimmed.is_empty() && !summary.contains(trimmed) {
                            if !summary.is_empty() {
                                summary.push_str("\n\n");
                            }
                            summary.push_str(trimmed);
                        }
                    }
                }
                _ => {
                    if !merged_value.contains_key(k) {
                        merged_value.insert(k.clone(), v.clone());
                    }
                }
            }
        }
    }
}

fn extract_text_from_value(v: &Value) -> Option<String> {
    if let Some(s) = v.as_str() {
        Some(s.to_owned())
    } else if let Some(arr) = v.as_array() {
        let mut flattened = String::new();
        for item in arr {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                if !flattened.is_empty() {
                    flattened.push_str(" ");
                }
                flattened.push_str(text);
                if let Some(src) = item.get("source").and_then(|s| s.as_str()) {
                    flattened.push_str(&format!(" ({})", src));
                }
            } else if let Some(s) = item.as_str() {
                if !flattened.is_empty() {
                    flattened.push_str(" ");
                }
                flattened.push_str(s);
            }
        }
        if flattened.is_empty() {
            Some(v.to_string())
        } else {
            Some(flattened)
        }
    } else {
        None
    }
}
