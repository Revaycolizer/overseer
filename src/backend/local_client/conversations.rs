use super::LocalClient;
use crate::backend::client::ConversationClient;
use crate::backend::Conversation;

#[async_trait::async_trait]
impl ConversationClient for LocalClient {
    async fn get_conversations(&self, limit: i64) -> anyhow::Result<Vec<Conversation>> {
        crate::backend::get_conversations(&self.db, limit).await
    }

    async fn add_conversation(&self, title: &str) -> anyhow::Result<i64> {
        crate::backend::add_conversation(&self.db, title).await
    }

    async fn update_conversation_title(&self, id: i64, title: &str) -> anyhow::Result<()> {
        crate::backend::update_conversation_title(&self.db, id, title).await
    }

    async fn delete_conversation(&self, id: i64) -> anyhow::Result<()> {
        crate::backend::delete_conversation(&self.db, id).await
    }

    async fn search_messages(
        &self,
        query: &str,
    ) -> anyhow::Result<Vec<crate::backend::ConversationSearchResult>> {
        crate::backend::conversations::search_messages(&self.db, query).await
    }
}
