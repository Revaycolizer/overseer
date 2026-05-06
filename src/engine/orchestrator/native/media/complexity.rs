// src/engine/orchestrator/native/media/complexity.rs
use std::path::Path;

/// Heuristics to determine if a media file is "complex" and needs Vision Assistant.
pub fn is_complex(path: &Path, native_text: Option<&str>) -> bool {
    let extension = path.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    // 1. If it's an image, it's inherently complex and requires Vision insights
    if matches!(extension.as_str(), "png" | "jpg" | "jpeg" | "webp" | "gif") {
        return true;
    }

    // 2. If native extraction returned very little text but the file is large
    if let Some(text) = native_text {
        let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        
        // Threshold: < 100 characters but > 500KB suggests it's a scanned/heavy document
        if text.trim().len() < 100 && file_size > 500_000 {
            return true;
        }
    } else {
        // Native extraction failed completely
        return true;
    }

    false
}
