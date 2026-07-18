use crate::backend::client::MessageClient;
use crate::backend::{Message, Attachment};
use super::LocalClient;

#[async_trait::async_trait]
impl MessageClient for LocalClient {
    async fn get_messages(&self, conversation_id: i64) -> anyhow::Result<Vec<Message>> {
        crate::backend::get_messages(&self.db, conversation_id).await
    }

    async fn get_messages_page(&self, conversation_id: i64, before_id: Option<i64>, limit: u32) -> anyhow::Result<Vec<Message>> {
        crate::backend::get_messages_page(&self.db, conversation_id, before_id, limit).await
    }

    async fn add_message(&self, conversation_id: i64, role: &str, content: &str, hide_evidence: bool) -> anyhow::Result<i64> {
        crate::backend::add_message(&self.db, conversation_id, role, content, hide_evidence).await
    }

    async fn get_attachments(&self, message_id: i64) -> anyhow::Result<Vec<Attachment>> {
        crate::backend::get_attachments(&self.db, message_id).await
    }

    async fn add_attachment(&self, message_id: i64, filename: &str, file_path: &str, content_type: &str, size: i64) -> anyhow::Result<i64> {
        crate::backend::add_attachment(&self.db, message_id, filename, file_path, content_type, size).await
    }

    async fn update_attachment_content(&self, attachment_id: i64, content: &str) -> anyhow::Result<()> {
        crate::backend::update_attachment_content(&self.db, attachment_id, content).await
    }
}
