use dioxus::prelude::document::eval;

pub fn clean_content(content: &str) -> String {
    let trimmed = content.trim();

    // 1. Reject raw tool calls — agent sent JSON instead of a final answer
    if trimmed.starts_with('{') && trimmed.contains("\"tool\"") {
        return "Agent sent a tool call incorrectly. Check logs.".to_string();
    }

    // 2. Handle ---CHART_XXX--- markers produced by the charting tool.
    //    The charting tool always returns a clean marker on its own line.
    //    We only need to clean up any stray separator lines that appear
    //    before the marker in the conversation history.
    if let Some(pos) = trimmed.find("---CHART_") {
        let before = trimmed[..pos]
            .trim()
            .trim_end_matches("---")
            .trim_end_matches("--")
            .trim();

        let from_marker = &trimmed[pos..];
        let first_newline = from_marker.find('\n').unwrap_or(from_marker.len());
        let marker_line = from_marker[..first_newline].trim();

        // The payload follows the marker line — strip any code fence the model added
        let rest = from_marker[first_newline..].trim();
        let inner = if rest.starts_with("```") {
            let after_fence = rest.lines().skip(1).collect::<Vec<_>>().join("\n");
            if let Some(end) = after_fence.rfind("```") {
                after_fence[..end].trim().to_string()
            } else {
                after_fence.trim().to_string()
            }
        } else {
            // Strip a trailing --- separator if present
            if let Some(end) = rest.rfind("\n---\n").or_else(|| rest.rfind("\n---")) {
                rest[..end].trim().to_string()
            } else {
                rest.trim().to_string()
            }
        };

        return if before.is_empty() {
            format!("{}\n{}", marker_line, inner)
        } else {
            format!("{}\n{}\n{}", before, marker_line, inner)
        };
    }

    // 3. Everything else is plain text — return as-is.
    //    The dispatcher guarantees that charts are returned directly from the
    //    charting tool as a clean ---CHART_SVG--- block, never mixed with prose.
    trimmed.to_string()
}

pub fn copy_to_clipboard(text: String) {
    let escaped = text
        .replace('\\', "\\\\")
        .replace('\'', "\\'")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    let eval_str = format!("navigator.clipboard.writeText('{}')", escaped);
    let _ = eval(&eval_str);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chart_marker_no_text_before() {
        let svg = r#"<svg xmlns="http://www.w3.org/2000/svg" baseProfile="full" viewBox="0 0 800 500"><rect width="800" height="500"/></svg>"#;
        let content = format!("---CHART_SVG---\n{}", svg);
        let cleaned = clean_content(&content);
        assert!(cleaned.starts_with("---CHART_SVG---"));
        assert!(cleaned.contains("<svg"));
    }

    #[test]
    fn test_chart_marker_with_text_before() {
        let svg =
            r#"<svg xmlns="http://www.w3.org/2000/svg" baseProfile="full" viewBox="0 0 800 500"/>"#;
        let content = format!(
            "Next.js grew from 1.8% to 3.0%.\n\n---\n---CHART_SVG---\n{}",
            svg
        );
        let cleaned = clean_content(&content);
        assert!(cleaned.starts_with("Next.js grew"));
        assert!(cleaned.contains("---CHART_SVG---"));
        // Stray separator dashes should be stripped
        assert!(!cleaned.contains("\n---\n---CHART"));
    }

    #[test]
    fn test_plain_text_passthrough() {
        let content = "Here is some analysis about Next.js growth.";
        let cleaned = clean_content(content);
        assert_eq!(cleaned, content);
    }

    #[test]
    fn test_tool_call_rejected() {
        let content = r#"{"tool": "charting", "input": {}}"#;
        let cleaned = clean_content(content);
        assert!(cleaned.contains("Agent sent a tool call incorrectly"));
    }

    #[test]
    fn test_no_svg_fallback() {
        // Raw SVG in the message must NOT be extracted — plain text passthrough only
        let content = r#"Here is a chart: <svg width="800"><rect/></svg> end."#;
        let cleaned = clean_content(content);
        assert!(!cleaned.contains("---CHART_SVG---"));
        assert_eq!(cleaned, content.trim());
    }
}
