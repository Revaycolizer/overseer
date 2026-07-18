use crate::backend::client::*;
use crate::backend::*;
use super::RemoteClient;

#[async_trait::async_trait]
impl WorkItemClient for RemoteClient {
    async fn create_work_item(&self, cid: i64, t: &str, b: Option<&str>, s: &str, k: &str, d: Option<&str>, p: &str, sa: Option<&str>, m: Option<&str>) -> anyhow::Result<i64> { Ok(self.http_client.post(format!("{}/api/work_items", self.base_url)).json(&serde_json::json!({ "conversation_id": cid, "title": t, "brief": b, "status": s, "surface_kind": k, "assigned_domain": d, "prompt": p, "source_artifact_ids_json": sa, "metadata_json": m })).send().await?.json().await?) }
    async fn get_work_items_for_conversation(&self, cid: i64, lim: i64) -> anyhow::Result<Vec<WorkItem>> { Ok(self.http_client.get(format!("{}/api/work_items?conversation_id={}&limit={}", self.base_url, cid, lim)).send().await?.json().await?) }
    async fn update_work_item_status(&self, id: i64, s: &str) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/work_items/update_status", self.base_url)).json(&serde_json::json!({ "id": id, "status": s })).send().await?; Ok(()) }
    async fn update_work_item_runtime(&self, id: i64, s: &str, m: Option<&str>) -> anyhow::Result<()> { self.http_client.post(format!("{}/api/work_items/update_runtime", self.base_url)).json(&serde_json::json!({ "id": id, "status": s, "metadata_json": m })).send().await?; Ok(()) }
    async fn log_work_item_event(&self, wid: i64, rid: Option<i64>, t: &str, s: Option<&str>, sm: Option<&str>, m: Option<&str>) -> anyhow::Result<i64> { Ok(self.http_client.post(format!("{}/api/work_items/event", self.base_url)).json(&serde_json::json!({ "work_item_id": wid, "execution_run_id": rid, "event_type": t, "status": s, "summary": sm, "metadata_json": m })).send().await?.json().await?) }
    async fn get_work_item_events(&self, wid: i64, limit: i64) -> anyhow::Result<Vec<WorkItemEvent>> { Ok(self.http_client.get(format!("{}/api/work_items/{}/events?limit={}", self.base_url, wid, limit)).send().await?.json().await?) }
}

#[async_trait::async_trait]
impl ArtifactClient for RemoteClient {
    async fn create_artifact(&self, cid: i64, execution_run_id: Option<i64>, d: Option<&str>, st: Option<&str>, k: &str, l: &str, c: Option<&str>, fp: Option<&str>, ct: Option<&str>, m: Option<&str>) -> anyhow::Result<i64> { Ok(self.http_client.post(format!("{}/api/artifacts", self.base_url)).json(&serde_json::json!({ "conversation_id": cid, "execution_run_id": execution_run_id, "domain": d, "step_text": st, "kind": k, "label": l, "content": c, "file_path": fp, "content_type": ct, "metadata_json": m })).send().await?.json().await?) }
    async fn get_artifacts_for_conversation(&self, cid: i64, lim: i64) -> anyhow::Result<Vec<Artifact>> { Ok(self.http_client.get(format!("{}/api/artifacts?conversation_id={}&limit={}", self.base_url, cid, lim)).send().await?.json().await?) }
    async fn get_artifacts_by_ids(&self, ids: &[i64]) -> anyhow::Result<Vec<Artifact>> { Ok(self.http_client.post(format!("{}/api/artifacts/by_ids", self.base_url)).json(&ids).send().await?.json().await?) }
    async fn get_artifacts_for_execution_run(&self, rid: i64) -> anyhow::Result<Vec<Artifact>> { Ok(self.http_client.get(format!("{}/api/artifacts/by_run?execution_run_id={}", self.base_url, rid)).send().await?.json().await?) }
}
