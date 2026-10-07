use serde_json::{json, Value};

pub fn process_unix_search_results(file_lines: Vec<String>, query: &str) -> Vec<Value> {
    let lower_query = query.to_lowercase();
    let mut tokens: Vec<String> = lower_query
        .split(|c: char| c.is_whitespace() || matches!(c, '?' | '.' | ',' | ':' | ';' | '\\' | '/'))
        .filter(|token| {
            token.len() >= 3
                && !matches!(
                    *token,
                    "find" | "path" | "file" | "files" | "pc" | "computer" | "my" | "the" | "in"
                )
        })
        .map(|token| token.to_string())
        .collect();

    if tokens.is_empty() {
        tokens.push(lower_query.clone());
    }

    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for line in file_lines {
        let path = line.trim();
        if path.is_empty() {
            continue;
        }

        let normalized = path.to_lowercase();
        if !seen.insert(normalized.clone()) {
            continue;
        }

        let name = std::path::Path::new(path)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(path)
            .to_string();
        let name_lower = name.to_lowercase();

        let matched = tokens
            .iter()
            .any(|token| name_lower.contains(token) || normalized.contains(token));
        if !matched {
            continue;
        }

        let mut score = 0;
        if name_lower == lower_query {
            score += 200;
        }
        for token in &tokens {
            if name_lower == *token {
                score += 80;
            }
            if name_lower.contains(token) {
                score += 30;
            }
            if normalized.contains(token) {
                score += 10;
            }
        }
        if normalized.contains("/overseer/src/") {
            score += 40;
        }
        if normalized.contains("/desktop/") {
            score += 5;
        }

        let directory = std::path::Path::new(path)
            .parent()
            .map(|value| value.to_string_lossy().to_string())
            .unwrap_or_default();
        let extension = std::path::Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| format!(".{}", value))
            .unwrap_or_default();

        results.push(json!({
            "name": name,
            "path": path,
            "directory": directory,
            "extension": extension,
            "score": score
        }));
    }

    results.sort_by(|a, b| {
        let score_a = a.get("score").and_then(|value| value.as_i64()).unwrap_or(0);
        let score_b = b.get("score").and_then(|value| value.as_i64()).unwrap_or(0);
        score_b.cmp(&score_a).then_with(|| {
            a.get("path")
                .and_then(|value| value.as_str())
                .cmp(&b.get("path").and_then(|value| value.as_str()))
        })
    });
    results.truncate(20);
    results
}
