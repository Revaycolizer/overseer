pub fn clean_content(content: &str) -> String {
    let re = regex::Regex::new(r"\[E[0-9]+\]").unwrap();
    let content = re.replace_all(content, "").to_string();
    let trimmed = content.trim();
    if (trimmed.starts_with('{') && trimmed.ends_with('}')) || (trimmed.starts_with('[') && trimmed.ends_with(']')) {
        if serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
            return trimmed.to_string(); // Don't clean JSON structured responses
        }
    }

    let without_markers = content.lines().filter(|line| {
        let t = line.trim();
        if t == "ARTIFACT_CONTEXT:" || t == "WORK_ITEM_CONTEXT:" { return false; }
        !(t.contains("[[ATTACH_IMAGE:") || t.contains("[[ATTACH_FILE:") || t.contains("[[ARTIFACT_REF:") || t.contains("[[WORK_ITEM_REF:"))
    }).collect::<Vec<_>>().join("\n");

    let trimmed = without_markers.trim();
    if trimmed.starts_with('{') && trimmed.contains("\"tool\"") {
        return "Agent sent a tool call incorrectly. Check logs.".to_string();
    }

    if let Some(pos) = trimmed.find("---CHART_") {
        let before = trimmed[..pos].trim().trim_end_matches("---").trim_end_matches("--").trim();
        let from_m = &trimmed[pos..];
        let first_nl = from_m.find('\n').unwrap_or(from_m.len());
        let marker = from_m[..first_nl].trim();
        let rest = from_m[first_nl..].trim();
        let inner = if rest.starts_with("```") {
            let after_f = rest.lines().skip(1).collect::<Vec<_>>().join("\n");
            if let Some(end) = after_f.rfind("```") { after_f[..end].trim().to_string() } else { after_f.trim().to_string() }
        } else {
            if let Some(end) = rest.rfind("\n---\n").or_else(|| rest.rfind("\n---")) { rest[..end].trim().to_string() } else { rest.trim().to_string() }
        };
        return if before.is_empty() { format!("{}\n{}", marker, inner) } else { format!("{}\n{}\n{}", before, marker, inner) };
    }
    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_grounding_markers() {
        let input = "Malaria is a disease [E1]. It is transmitted by mosquitoes [E2].";
        let expected = "Malaria is a disease . It is transmitted by mosquitoes .";
        // Note: the executor logic also handles double space cleanup, 
        // but clean_content just strips the regex.
        assert_eq!(clean_content(input), expected);
    }
}
