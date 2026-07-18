/// Strip a JSON object that's been embedded as plain text inside a
/// message string. Some small models (qwen2.5 7b in particular) wrap
/// their structured response as plain text inside the assistant
/// `messages[].message` field, so the renderer ends up showing both
/// the prose answer and the JSON shell.
///
/// Walks the string looking for the first `{` and the matching `}`,
/// verifies the candidate parses as JSON, and removes it. Only the
/// first matching object is stripped; nested objects are handled by
/// the brace counter.
pub fn strip_embedded_json(text: &str) -> String {
    let bytes = text.as_bytes();
    let Some(start) = bytes.iter().position(|&b| b == b'{') else {
        return text.to_string();
    };

    let mut depth: i32 = 0;
    let mut end: Option<usize> = None;
    for (i, &b) in bytes.iter().enumerate().skip(start) {
        match b {
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }

    let Some(end_idx) = end else { return text.to_string(); };

    let candidate = &text[start..=end_idx];
    if serde_json::from_str::<serde_json::Value>(candidate).is_err() {
        return text.to_string();
    }

    let mut out = String::with_capacity(text.len());
    out.push_str(text[..start].trim_end());
    let after = text[end_idx + 1..].trim_start();
    if !after.is_empty() {
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(after);
    }
    out
}