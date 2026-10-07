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

    let Some(end_idx) = end else {
        return text.to_string();
    };

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

    // JSON was actually stripped, so the leftover is often a broken fragment
    // (e.g. the model emitted `{...tool call...}' as requested .`, leaving
    // "' as requested ."). Tidy the remainder so the reader never sees garbage.
    tidy_leftover(&out)
}

/// Clean up the prose left after an embedded JSON object is removed: drop stray
/// leading punctuation/quotes, collapse whitespace, remove spaces before
/// sentence punctuation, and capitalize the first letter. Falls back to a short
/// neutral message when nothing meaningful remains.
fn tidy_leftover(s: &str) -> String {
    let trimmed = s.trim();
    // Strip stray leading punctuation left dangling by the removed JSON.
    let trimmed = trimmed
        .trim_start_matches(|c: char| {
            matches!(
                c,
                '\'' | '"' | '`' | ',' | ':' | ';' | '-' | '.' | ' ' | '\t' | '\n'
            )
        })
        .trim();

    // Collapse runs of whitespace to single spaces.
    let mut collapsed = String::with_capacity(trimmed.len());
    let mut prev_space = false;
    for c in trimmed.chars() {
        if c.is_whitespace() {
            if !prev_space {
                collapsed.push(' ');
            }
            prev_space = true;
        } else {
            collapsed.push(c);
            prev_space = false;
        }
    }
    let collapsed = collapsed
        .trim()
        .replace(" .", ".")
        .replace(" ,", ",")
        .replace(" !", "!")
        .replace(" ?", "?");

    if collapsed.is_empty() {
        return "Done.".to_string();
    }

    // Capitalize the first character.
    let mut chars = collapsed.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => collapsed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leaves_plain_prose_untouched() {
        let s = "I created a plan and here are the three steps.";
        assert_eq!(strip_embedded_json(s), s);
    }

    #[test]
    fn does_not_touch_text_without_json() {
        // No braces at all -> returned verbatim (no capitalization/trimming).
        let s = "as requested by the user";
        assert_eq!(strip_embedded_json(s), s);
    }

    #[test]
    fn strips_leading_tool_call_and_tidies_fragment() {
        // The exact failure mode: a tool-call JSON blob followed by a fragment.
        let s = r#"{"observation":"ok","tool":"write_file","input":{"path":"/tmp/x/proof.txt"}}' as requested ."#;
        assert_eq!(strip_embedded_json(s), "As requested.");
    }

    #[test]
    fn json_only_becomes_neutral_message() {
        let s = r#"{"tool":"write_file","input":{"path":"/tmp/x"}}"#;
        assert_eq!(strip_embedded_json(s), "Done.");
    }

    #[test]
    fn keeps_real_prose_around_json() {
        let s = r#"I wrote the report. {"tool":"noop"} Let me know if you need changes."#;
        assert_eq!(
            strip_embedded_json(s),
            "I wrote the report. Let me know if you need changes."
        );
    }

    #[test]
    fn non_json_braces_are_preserved() {
        // A brace that doesn't parse as JSON must not be stripped.
        let s = "Use the pattern {value} in your template.";
        assert_eq!(strip_embedded_json(s), s);
    }
}
