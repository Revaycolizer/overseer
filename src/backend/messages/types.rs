use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub id: i64,
    pub conversation_id: i64,
    pub role: String,
    pub content: String,
    pub created_at: chrono::NaiveDateTime,
    pub hide_evidence: bool,
    #[serde(default)]
    pub attachments: Vec<Attachment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq, Default)]
pub struct Attachment {
    pub id: i64,
    pub message_id: i64,
    pub filename: String,
    pub file_path: String,
    pub content_type: String,
    pub size: i64,
    pub extracted_content: Option<String>,
}
