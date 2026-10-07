use crate::backend::{Attachment, Message};

#[async_trait::async_trait]
pub trait MessageClient {
    async fn get_messages(&self, conversation_id: i64) -> anyhow::Result<Vec<Message>>;
    async fn get_messages_page(
        &self,
        conversation_id: i64,
        before_id: Option<i64>,
        limit: u32,
    ) -> anyhow::Result<Vec<Message>>;
    async fn add_message(
        &self,
        conversation_id: i64,
        role: &str,
        content: &str,
        hide_evidence: bool,
    ) -> anyhow::Result<i64>;
    async fn get_attachments(&self, message_id: i64) -> anyhow::Result<Vec<Attachment>>;
    async fn add_attachment(
        &self,
        message_id: i64,
        filename: &str,
        file_path: &str,
        content_type: &str,
        size: i64,
    ) -> anyhow::Result<i64>;
    async fn update_attachment_content(
        &self,
        attachment_id: i64,
        content: &str,
    ) -> anyhow::Result<()>;
    async fn delete_message(&self, id: i64) -> anyhow::Result<()>;
}
