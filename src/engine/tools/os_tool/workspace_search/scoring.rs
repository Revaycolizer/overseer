pub fn tokenize_file_query(query: &str) -> Vec<String> {
    let mut tokens: Vec<String> = query
        .to_lowercase()
        .split(|ch: char| matches!(ch, ' ' | '?' | '.' | ',' | ':' | ';' | '\\' | '/'))
        .map(str::trim)
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
        let trimmed = query.trim().to_lowercase();
        if !trimmed.is_empty() {
            tokens.push(trimmed);
        }
    }

    tokens
}

pub fn score_file_match(
    path_lower: &str,
    name_lower: &str,
    query_lower: &str,
    tokens: &[String],
) -> i32 {
    let mut score = 0;

    if name_lower == query_lower {
        score += 200;
    }

    for token in tokens {
        if name_lower == token {
            score += 80;
        }
        if name_lower.contains(token) {
            score += 30;
        }
        if path_lower.contains(token) {
            score += 10;
        }
    }

    if path_lower.contains("\\overseer\\src\\") || path_lower.contains("/overseer/src/") {
        score += 40;
    }
    if path_lower.contains("\\desktop\\") || path_lower.contains("/desktop/") {
        score += 5;
    }

    score
}
