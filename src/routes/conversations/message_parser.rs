use crate::routes::conversations::content_cleaner::clean_content;

pub struct ParsedMessage {
    pub display_text: String,
    pub attachments: Vec<String>,
}

pub fn parse_message(content: &str) -> ParsedMessage {
    let mut attachments = Vec::new();
    for line in content.lines() {
        let t = line.trim();
        if let Some(inner) = t.strip_prefix("[[ATTACH_FILE:").and_then(|s| s.strip_suffix("]]")) { attachments.push(inner.to_string()); }
        else if let Some(inner) = t.strip_prefix("[[ATTACH_IMAGE:").and_then(|s| s.strip_suffix("]]")) { attachments.push(inner.to_string()); }
    }
    ParsedMessage { display_text: clean_content(content), attachments }
}

pub fn extract_agent_attachments(content: &str) -> Vec<String> {
    content.lines().filter_map(|line| {
        let t = line.trim();
        t.strip_prefix("[[ATTACH_FILE:").and_then(|s| s.strip_suffix("]]")).map(|path| path.to_string())
    }).collect()
}
