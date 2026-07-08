use crate::backend::client::MessageClient;
use crate::backend::{Message, Attachment};
use super::RemoteClient;

#[async_trait::async_trait]
impl MessageClient for RemoteClient {
    async fn get_messages(&self, cid: i64) -> anyhow::Result<Vec<Message>> {
        let url = format!("{}/api/messages?conversation_id={}", self.base_url, cid);
        let res = self.http_client.get(&url).send().await?.json::<Vec<Message>>().await?;
        Ok(res)
    }

    async fn add_message(&self, cid: i64, role: &str, content: &str, hide_evidence: bool) -> anyhow::Result<i64> {
        let url = format!("{}/api/messages", self.base_url);
        let payload = serde_json::json!({ "conversation_id": cid, "role": role, "content": content, "hide_evidence": hide_evidence });
        let res = self.http_client.post(&url).json(&payload).send().await?.json::<i64>().await?;
        Ok(res)
    }

    async fn get_attachments(&self, mid: i64) -> anyhow::Result<Vec<Attachment>> {
        let url = format!("{}/api/attachments?message_id={}", self.base_url, mid);
        let res = self.http_client.get(&url).send().await?.json::<Vec<Attachment>>().await?;
        Ok(res)
    }

    async fn add_attachment(&self, mid: i64, name: &str, path: &str, ct: &str, sz: i64) -> anyhow::Result<i64> {
        let url = format!("{}/api/attachments", self.base_url);
        let payload = serde_json::json!({ "message_id": mid, "filename": name, "file_path": path, "content_type": ct, "size": sz });
        let res = self.http_client.post(&url).json(&payload).send().await?.json::<i64>().await?;
        Ok(res)
    }

    async fn update_attachment_content(&self, id: i64, content: &str) -> anyhow::Result<()> {
        let url = format!("{}/api/attachments/update_content", self.base_url);
        let payload = serde_json::json!({ "id": id, "content": content });
        self.http_client.post(&url).json(&payload).send().await?;
        Ok(())
    }
}
