use crate::engine::tools::input::read_string_field;

pub fn extract_text_value(input: &serde_json::Value) -> Option<String> {
    if let Some(value) = read_string_field(input, &["text", "value", "content"]) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    let raw = read_string_field(input, &["input"])?;
    let lower = raw.to_lowercase();
    for prefix in ["type ", "enter text ", "input text "] {
        if lower.starts_with(prefix) {
            let extracted = raw[prefix.len()..].trim();
            if !extracted.is_empty() {
                return Some(extracted.to_string());
            }
        }
    }

    None
}
