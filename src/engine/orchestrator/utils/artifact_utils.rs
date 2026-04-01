pub fn infer_artifact_kind_and_content_type(value: &str) -> (String, Option<String>) {
    let t = value.trim();
    if t.starts_with("http://") || t.starts_with("https://") {
        return ("link".to_string(), Some("text/uri-list".to_string()));
    }
    let path = std::path::Path::new(t);
    if path.is_absolute() || path.exists() || (path.extension().is_some() && (t.contains('\\') || t.contains('/') || t.contains('.'))) {
        let ct = mime_guess::from_path(path).first_or_octet_stream().essence_str().to_string();
        let kind = if ct.starts_with("image/") { "image" } else { "file" };
        return (kind.to_string(), Some(ct));
    }
    if serde_json::from_str::<serde_json::Value>(t).is_ok() {
        return ("record".to_string(), Some("application/json".to_string()));
    }
    ("note".to_string(), Some("text/plain".to_string()))
}

pub fn infer_artifact_label(kind: &str, step: &str, value: &str) -> String {
    let t = value.trim();
    let path = std::path::Path::new(t);
    if matches!(kind, "image" | "file") {
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            return if kind == "image" && name.to_ascii_lowercase().contains("desktop") { "Desktop Screenshot".to_string() } else { name.to_string() };
        }
    }
    if kind == "link" { return "Linked Artifact".to_string(); }
    let compact = step.trim();
    if !compact.is_empty() { return compact.chars().take(72).collect::<String>().trim().to_string(); }
    t.chars().take(72).collect::<String>().trim().to_string()
}
