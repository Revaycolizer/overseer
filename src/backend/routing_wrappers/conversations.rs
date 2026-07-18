use crate::db::Database;
use crate::backend::{Conversation, Message, Attachment};
use crate::backend::client::{ConversationClient, MessageClient};

pub async fn get_conversations(db: &Database, limit: i64) -> anyhow::Result<Vec<Conversation>> {
    if db.is_remote() {
        return db.remote_client().get_conversations(limit).await;
    }
    crate::backend::conversations::get_conversations(db).await
}

pub async fn add_conversation(db: &Database, title: &str) -> anyhow::Result<i64> {
    if db.is_remote() {
        return db.remote_client().add_conversation(title).await;
    }
    crate::backend::conversations::add_conversation(db, title).await
}

pub async fn update_conversation_title(db: &Database, id: i64, title: &str) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().update_conversation_title(id, title).await;
    }
    crate::backend::conversations::update_conversation_title(db, id, title).await
}

pub async fn delete_conversation(db: &Database, id: i64) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().delete_conversation(id).await;
    }
    crate::backend::conversations::delete_conversation(db, id).await
}

pub async fn get_messages(db: &Database, conversation_id: i64) -> anyhow::Result<Vec<Message>> {
    if db.is_remote() {
        return db.remote_client().get_messages(conversation_id).await;
    }
    crate::backend::messages::get_messages(db, conversation_id).await
}

pub async fn get_messages_page(db: &Database, conversation_id: i64, before_id: Option<i64>, limit: u32) -> anyhow::Result<Vec<Message>> {
    if db.is_remote() {
        return db.remote_client().get_messages_page(conversation_id, before_id, limit).await;
    }
    crate::backend::messages::get_messages_page(db, conversation_id, before_id, limit).await
}

pub async fn add_message(db: &Database, conversation_id: i64, role: &str, content: &str, hide_evidence: bool) -> anyhow::Result<i64> {
    if db.is_remote() {
        return db.remote_client().add_message(conversation_id, role, content, hide_evidence).await;
    }
    crate::backend::messages::add_message(db, conversation_id, role, content, hide_evidence).await
}

pub async fn get_attachments(db: &Database, message_id: i64) -> anyhow::Result<Vec<Attachment>> {
    if db.is_remote() {
        return db.remote_client().get_attachments(message_id).await;
    }
    crate::backend::messages::get_attachments(db, message_id).await
}

pub async fn add_attachment(db: &Database, message_id: i64, filename: &str, file_path: &str, content_type: &str, size: i64) -> anyhow::Result<i64> {
    if db.is_remote() {
        return db.remote_client().add_attachment(message_id, filename, file_path, content_type, size).await;
    }
    crate::backend::messages::add_attachment(db, message_id, filename, file_path, content_type, size).await
}

pub async fn update_attachment_content(db: &Database, attachment_id: i64, content: &str) -> anyhow::Result<()> {
    if db.is_remote() {
        return db.remote_client().update_attachment_content(attachment_id, content).await;
    }
    crate::backend::messages::update_attachment_content(db, attachment_id, content).await
}
