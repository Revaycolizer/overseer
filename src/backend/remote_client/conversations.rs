use crate::backend::client::ConversationClient;
use crate::backend::Conversation;
use super::RemoteClient;

#[async_trait::async_trait]
impl ConversationClient for RemoteClient {
    async fn get_conversations(&self, limit: i64) -> anyhow::Result<Vec<Conversation>> {
        let url = format!("{}/api/conversations?limit={}", self.base_url, limit);
        let res = self.http_client.get(&url).send().await?.json::<Vec<Conversation>>().await?;
        Ok(res)
    }

    async fn add_conversation(&self, title: &str) -> anyhow::Result<i64> {
        let url = format!("{}/api/conversations", self.base_url);
        let payload = serde_json::json!({ "title": title });
        let res = self.http_client.post(&url).json(&payload).send().await?.json::<i64>().await?;
        Ok(res)
    }

    async fn update_conversation_title(&self, id: i64, title: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/conversations/update_title", self.base_url);
        let payload = serde_json::json!({ "id": id, "title": title });
        self.http_client.post(&url).json(&payload).send().await?;
        Ok(())
    }

    async fn delete_conversation(&self, id: i64) -> anyhow::Result<()> {
        let url = format!("{}/api/conversations/delete", self.base_url);
        let payload = serde_json::json!({ "id": id });
        self.http_client.post(&url).json(&payload).send().await?;
        Ok(())
    }
}
