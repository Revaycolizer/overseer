use std::collections::HashSet;
use std::path::{Path, PathBuf, MAIN_SEPARATOR_STR};

pub(crate) fn is_image_file_path(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
    )
}

fn normalize_candidate(line: &str) -> Option<&str> {
    let trimmed = line
        .trim()
        .trim_start_matches("- ")
        .trim_start_matches("* ")
        .trim_matches('`')
        .trim_matches('"')
        .trim_end_matches(['.', ',', ';']);

    if trimmed.is_empty() || trimmed.ends_with(':') {
        return None;
    }

    let lower = trimmed.to_lowercase();
    let is_file = [".pdf", ".docx", ".doc", ".xlsx", ".xls", ".csv", ".zip", ".json", ".txt", ".png", ".jpg", ".jpeg", ".webp"]
        .iter()
        .any(|ext| lower.ends_with(ext));

    if is_file { Some(trimmed) } else { None }
}

fn extract_markdown_image_target(line: &str) -> Option<String> {
    let start = line.find("](")?;
    let end = line[start + 2..].find(')')?;
    Some(line[start + 2..start + 2 + end].trim().to_string())
}

fn extract_attach_marker_target(line: &str) -> Option<String> {
    let trimmed = line.trim();
    for prefix in ["[[ATTACH_IMAGE:", "[[ATTACH_FILE:"] {
        if let Some(start) = trimmed.find(prefix) {
            let rest = &trimmed[start + prefix.len()..];
            if let Some(end) = rest.find("]]") {
                let target = rest[..end].trim();
                if !target.is_empty() {
                    return Some(target.to_string());
                }
            }
        }
    }
    None
}

pub(crate) fn extract_local_file_paths_from_content(content: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let mut seen = HashSet::new();

    for line in content.lines() {
        let attach_marker = extract_attach_marker_target(line);
        let markdown_target = if line.contains("![") && line.contains("](") {
            extract_markdown_image_target(line)
        } else {
            None
        };

        let Some(candidate) = attach_marker
            .or(markdown_target)
            .or_else(|| normalize_candidate(line).map(str::to_string))
        else {
            continue;
        };

        let candidate = candidate
            .strip_prefix("file:///")
            .or_else(|| candidate.strip_prefix("file://"))
            .unwrap_or(&candidate)
            .replace('/', MAIN_SEPARATOR_STR);

        let path = PathBuf::from(candidate);
        let exists = path.is_absolute() && path.exists() || std::fs::metadata(&path).is_ok();

        if exists && seen.insert(path.clone()) {
            paths.push(path);
        }
    }

    paths
}
