use crate::backend::Conversation;

#[async_trait::async_trait]
pub trait ConversationClient {
    async fn get_conversations(&self, limit: i64) -> anyhow::Result<Vec<Conversation>>;
    async fn add_conversation(&self, title: &str) -> anyhow::Result<i64>;
    async fn update_conversation_title(&self, id: i64, title: &str) -> anyhow::Result<()>;
    async fn delete_conversation(&self, id: i64) -> anyhow::Result<()>;
}
