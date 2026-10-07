use crate::backend;
use crate::db::Database;

pub fn infer_native_computer_domain(query: &str) -> Option<String> {
    let l = query.to_lowercase();
    if l.contains("desktop")
        || l.contains("pc")
        || l.contains("computer")
        || l.contains("machine")
        || l.contains("system")
    {
        Some("computer".to_string())
    } else {
        None
    }
}

pub fn is_native_computer_fast_path_request(query: &str, domain: &str) -> bool {
    if !matches!(domain, "computer" | "desktop") {
        return false;
    }
    if infer_native_computer_domain(query).is_some() {
        return true;
    }
    let l = query.to_lowercase();
    l.contains("background task")
        || l.contains("process")
        || l.contains("screenshot")
        || l.contains("current tab")
        || l.contains("capture")
}

pub fn is_affirmative_follow_up(query: &str) -> bool {
    let n = query.trim().to_lowercase();
    [
        "continue", "yes", "go ahead", "proceed", "ok", "okay", "do it",
    ]
    .iter()
    .any(|&m| n == m || n.starts_with(&(m.to_owned() + " ")))
}

pub fn is_negative_follow_up(query: &str) -> bool {
    let n = query.trim().to_lowercase();
    [
        "no", "stop", "cancel", "don't", "dont", "n", "false", "wait",
    ]
    .iter()
    .any(|&m| n == m || n.starts_with(&(m.to_owned() + " ")))
}

pub async fn latest_assistant_image_attachment(
    db: &Database,
    conversation_id: i64,
) -> Option<backend::Attachment> {
    let messages = backend::get_messages(db, conversation_id).await.ok()?;
    for msg in messages.into_iter().rev().filter(|m| m.role == "assistant") {
        if let Some(att) = msg
            .attachments
            .into_iter()
            .rev()
            .find(|a| a.content_type.starts_with("image/"))
        {
            return Some(att);
        }
    }
    None
}

pub async fn resolve_screenshot_follow_up(
    db: &Database,
    conversation_id: i64,
    query: &str,
) -> Option<(String, String)> {
    let intent = if query.to_lowercase().contains("where") {
        "locate"
    } else if query.to_lowercase().contains("send") || query.to_lowercase().contains("show") {
        "resend"
    } else {
        return None;
    };
    let att = latest_assistant_image_attachment(db, conversation_id).await?;
    let path = att.file_path.trim();
    if path.is_empty() {
        return None;
    }
    let resp = if intent == "locate" {
        format!(
            "The latest screenshot is saved here:\n\n{}\n\nFilename: {}",
            path, att.filename
        )
    } else {
        format!(
            "Here is the latest screenshot.\n\n[[ATTACH_IMAGE:{}]]",
            path
        )
    };
    Some((
        resp,
        serde_json::json!({"intent": intent, "path": path, "filename": att.filename}).to_string(),
    ))
}
