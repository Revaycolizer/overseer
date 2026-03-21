use dioxus::prelude::document::eval;

pub fn clean_content(content: &str) -> String {
    let trimmed = content.trim();

    // 1. Reject raw tool calls
    if trimmed.starts_with('{') && trimmed.contains("\"tool\"") {
        return "Agent sent a tool call incorrectly. Check logs.".to_string();
    }

    // 2. Look for ---CHART_XXX--- marker anywhere in the content
    if let Some(pos) = trimmed.find("---CHART_") {
        let before = trimmed[..pos].trim()
            .trim_end_matches("---")
            .trim_end_matches("--")
            .trim();

        let from_marker = &trimmed[pos..];
        let first_newline = from_marker.find('\n').unwrap_or(from_marker.len());
        let marker_line = from_marker[..first_newline].trim();
        let rest = from_marker[first_newline..].trim();

        // Strip markdown code fences if present
        let inner = if rest.starts_with("```") {
            let after_fence = rest.lines().skip(1).collect::<Vec<_>>().join("\n");
            if let Some(end) = after_fence.rfind("```") {
                after_fence[..end].trim().to_string()
            } else {
                after_fence.trim().to_string()
            }
        } else {
            if let Some(end) = rest.rfind("\n---\n").or_else(|| rest.rfind("\n---")) {
                rest[..end].trim().to_string()
            } else {
                rest.trim().to_string()
            }
        };

        // Validate: reject agent-written broken SVGs
        // Charming SVGs always have xmlns; hand-written ones often have broken transforms
        if marker_line == "---CHART_SVG---" && is_broken_svg(&inner) {
            // Discard the broken SVG, return only the text before it
            return if before.is_empty() {
                "Chart generation failed — invalid SVG detected. Please try again.".to_string()
            } else {
                before.to_string()
            };
        }

        let inner = patch_viewbox(&inner);

        if before.is_empty() {
            return format!("{}\n{}", marker_line, inner);
        } else {
            return format!("{}\n{}\n{}", before, marker_line, inner);
        }
    }

    // 3. Handle bare ```svg blocks (agent forgot the marker)
    if let Some(pos) = trimmed.find("```svg").or_else(|| trimmed.find("```SVG")) {
        let before = trimmed[..pos].trim()
            .trim_end_matches("---")
            .trim();
        let after_fence = &trimmed[pos + 6..];
        let content = if let Some(end) = after_fence.find("```") {
            after_fence[..end].trim()
        } else {
            after_fence.trim()
        };

        if is_broken_svg(content) {
            return if before.is_empty() {
                "Chart generation failed — invalid SVG detected. Please try again.".to_string()
            } else {
                before.to_string()
            };
        }

        let content = patch_viewbox(content);
        if before.is_empty() {
            return format!("---CHART_SVG---\n{}", content);
        } else {
            return format!("{}\n---CHART_SVG---\n{}", before, content);
        }
    }

    // 4. Handle bare ```html blocks
    if let Some(pos) = trimmed.find("```html").or_else(|| trimmed.find("```HTML")) {
        let before = trimmed[..pos].trim()
            .trim_end_matches("---")
            .trim();
        let after_fence = &trimmed[pos + 7..];
        let content = if let Some(end) = after_fence.find("```") {
            after_fence[..end].trim()
        } else {
            after_fence.trim()
        };

        // If the html block contains an SVG, treat as SVG chart
        let lower = content.to_lowercase();
        let marker = if lower.contains("<svg") {
            // Validate it's not broken
            if is_broken_svg(content) {
                return if before.is_empty() {
                    "Chart generation failed — invalid SVG detected. Please try again.".to_string()
                } else {
                    before.to_string()
                };
            }
            "---CHART_SVG---"
        } else {
            "---CHART_HTML---"
        };

        let content = patch_viewbox(content);
        if before.is_empty() {
            return format!("{}\n{}", marker, content);
        } else {
            return format!("{}\n{}\n{}", before, marker, content);
        }
    }

    // 5. Fallback: extract raw HTML / SVG block if present
    let lower = trimmed.to_lowercase();
    let start_candidates = [
        lower.find("<!doctype"),
        lower.find("<html"),
        lower.find("<svg"),
        lower.find("<?xml"),
    ];
    let start_pos = start_candidates.iter().flatten().min().copied();

    if let Some(pos) = start_pos {
        let before = trimmed[..pos].trim()
            .trim_end_matches("---")
            .trim();
        let end_candidates = [
            lower.rfind("</html>").map(|e| e + 7),
            lower.rfind("</svg>").map(|e| e + 6),
        ];
        let end = end_candidates
            .iter()
            .flatten()
            .max()
            .copied()
            .unwrap_or(trimmed.len());

        if end > pos {
            let raw = &trimmed[pos..end];

            if is_broken_svg(raw) {
                return if before.is_empty() {
                    "Chart generation failed — invalid SVG detected. Please try again.".to_string()
                } else {
                    before.to_string()
                };
            }

            let content = patch_viewbox(raw);
            let marker = if lower[pos..].contains("<svg") {
                "---CHART_SVG---"
            } else {
                "---CHART_HTML---"
            };

            if before.is_empty() {
                return format!("{}\n{}", marker, content);
            } else {
                return format!("{}\n{}\n{}", before, marker, content);
            }
        }
    }

    trimmed.to_string()
}

/// Detect agent-written broken SVGs.
/// Charming SVGs always have xmlns and a proper viewBox.
/// Broken agent SVGs typically have:
/// - No xmlns attribute
/// - Root-level transform="translate(...)" with large offsets
/// - circle/path elements with huge negative coordinates
fn is_broken_svg(svg: &str) -> bool {
    if !svg.contains("<svg") {
        return false;
    }
    // Must have xmlns to be a valid charming SVG
    let has_xmlns = svg.contains("xmlns=");
    // Agent-written SVGs often have transform with large negative values at root
    let has_root_translate = svg.contains("transform=\"translate(")
        && (svg.contains("-309") || svg.contains("-255") || svg.contains("-710"));
    // Charming SVGs always have baseProfile="full"
    let has_base_profile = svg.contains("baseProfile=\"full\"");

    // If it has xmlns and baseProfile it's from charming — trust it
    if has_xmlns && has_base_profile {
        return false;
    }
    // If it has suspicious root transforms, it's broken
    if has_root_translate {
        return true;
    }
    // If it has no xmlns at all, it's hand-written — reject
    if !has_xmlns {
        return true;
    }

    false
}

/// Patch SVG for responsive rendering.
/// Charming SVGs already have viewBox — just make responsive.
/// Hand-written SVGs get a viewBox added from their width/height attrs.
fn patch_viewbox(svg: &str) -> String {
    if !svg.contains("<svg") {
        return svg.to_string();
    }
    if svg.contains("viewBox") {
        return svg.replacen(
            "<svg ",
            "<svg style=\"width:100%;height:100%;\" ",
            1,
        );
    }
    let w = extract_attr(svg, "width").unwrap_or("800".to_string());
    let h = extract_attr(svg, "height").unwrap_or("450".to_string());
    svg.replacen(
        "<svg ",
        &format!("<svg viewBox=\"0 0 {w} {h}\" style=\"width:100%;height:100%;\" "),
        1,
    )
}

/// Extract a simple quoted attribute value from an SVG/HTML tag.
fn extract_attr(s: &str, attr: &str) -> Option<String> {
    let needle = format!("{}=\"", attr);
    let start = s.find(&needle)? + needle.len();
    let end = s[start..].find('"')? + start;
    Some(s[start..end].to_string())
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

fn sanitize_context(context: &str) -> String {
    // Remove SVG blocks from conversation history
    let mut result = String::new();
    let mut skip_block = false;
    
    for line in context.lines() {
        let lower = line.to_lowercase();
        // Start skipping at SVG/HTML code fences
        if lower.contains("```svg") || lower.contains("```html") || lower.contains("```xml") {
            skip_block = true;
            continue;
        }
        // Stop skipping at closing fence
        if skip_block && line.trim() == "```" {
            skip_block = false;
            continue;
        }
        // Skip lines with raw SVG tags
        if skip_block || lower.contains("<svg") || lower.contains("</svg>") {
            continue;
        }
        result.push_str(line);
        result.push('\n');
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strips_trailing_dashes_before_marker() {
        let content = "Next.js grew from 1.8% to 3.0%.\n\n---\n---CHART_SVG---\n<svg xmlns=\"http://www.w3.org/2000/svg\" baseProfile=\"full\" viewBox=\"0 0 800 500\">...</svg>";
        let cleaned = clean_content(content);
        assert!(cleaned.starts_with("Next.js grew"));
        assert!(cleaned.contains("---CHART_SVG---"));
    }

    #[test]
    fn test_rejects_broken_svg() {
        let content = "<svg width=\"800\"><g transform=\"translate(-309 -710)\"><circle cx=\"100\" cy=\"100\" r=\"80\"></circle></g></svg>";
        assert!(is_broken_svg(content));
    }

    #[test]
    fn test_accepts_charming_svg() {
        let content = "<svg width=\"800\" height=\"500\" xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" baseProfile=\"full\" viewBox=\"0 0 800 500\">...</svg>";
        assert!(!is_broken_svg(content));
    }

    #[test]
    fn test_bare_svg_fence() {
        let content = "Here:\n```svg\n<svg xmlns=\"http://www.w3.org/2000/svg\" baseProfile=\"full\" width=\"800\" height=\"500\">...</svg>\n```";
        let cleaned = clean_content(content);
        assert!(cleaned.contains("---CHART_SVG---"));
    }
}