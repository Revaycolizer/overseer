pub fn normalize_summary(summary: &str) -> String {
    summary
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                trimmed.trim_start_matches('#').trim().to_string()
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

pub fn extract_artifact_path(item: &str) -> Option<String> {
    let lower = item.to_lowercase();
    
    // 1. Check for common file extensions (Images + Documents)
    let has_extension = lower.contains(".png") || lower.contains(".jpg") || lower.contains(".jpeg") || 
                        lower.contains(".webp") || lower.contains(".pdf") || lower.contains(".csv") ||
                        lower.contains(".xlsx") || lower.contains(".docx") || lower.contains(".xls") ||
                        lower.contains(".doc") || lower.contains(".txt") || lower.contains(".json") ||
                        lower.contains(".md") || lower.contains(".svg");

    if (has_extension && (item.contains('/') || item.contains('\\') || item.contains("assets/"))) {
        return Some(item.trim().to_string());
    }

    // 2. Try parsing as JSON object
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(item) {
        if let Some(path) = value.get("path").and_then(|v| v.as_str()) {
            return Some(path.to_string());
        }
    }

    // 3. Last resort: If it's a bare string that strictly starts with "assets/"
    if item.starts_with("assets/") {
        return Some(item.trim().to_string());
    }

    None
}

pub fn is_html_noise(value: &str) -> bool {
    let lower = value.to_lowercase();
    lower.contains("<img ") || lower.contains("```html") || lower.contains("<html")
}
