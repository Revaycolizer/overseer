use super::LocalClient;
use crate::backend::client::*;
use crate::backend::*;

#[async_trait::async_trait]
impl WorkItemClient for LocalClient {
    async fn create_work_item(
        &self,
        cid: i64,
        t: &str,
        b: Option<&str>,
        s: &str,
        k: &str,
        d: Option<&str>,
        p: &str,
        sa: Option<&str>,
        m: Option<&str>,
    ) -> anyhow::Result<i64> {
        crate::backend::work_items::create_work_item(&self.db, cid, t, b, s, k, d, p, sa, m).await
    }
    async fn get_work_items_for_conversation(
        &self,
        cid: i64,
        lim: i64,
    ) -> anyhow::Result<Vec<WorkItem>> {
        crate::backend::work_items::get_work_items_for_conversation(&self.db, cid, lim).await
    }
    async fn update_work_item_status(&self, id: i64, s: &str) -> anyhow::Result<()> {
        crate::backend::work_items::update_work_item_status(&self.db, id, s).await
    }
    async fn update_work_item_runtime(
        &self,
        id: i64,
        s: &str,
        m: Option<&str>,
    ) -> anyhow::Result<()> {
        crate::backend::work_items::update_work_item_runtime(&self.db, id, s, m).await
    }
    async fn log_work_item_event(
        &self,
        wid: i64,
        rid: Option<i64>,
        t: &str,
        s: Option<&str>,
        sm: Option<&str>,
        m: Option<&str>,
    ) -> anyhow::Result<i64> {
        crate::backend::work_items::log_work_item_event(&self.db, wid, rid, t, s, sm, m).await
    }
    async fn get_work_item_events(
        &self,
        wid: i64,
        limit: i64,
    ) -> anyhow::Result<Vec<WorkItemEvent>> {
        crate::backend::work_items::get_work_item_events(&self.db, wid, limit).await
    }
}

#[async_trait::async_trait]
impl ArtifactClient for LocalClient {
    async fn create_artifact(
        &self,
        cid: i64,
        execution_run_id: Option<i64>,
        d: Option<&str>,
        st: Option<&str>,
        k: &str,
        l: &str,
        c: Option<&str>,
        fp: Option<&str>,
        ct: Option<&str>,
        m: Option<&str>,
    ) -> anyhow::Result<i64> {
        crate::backend::artifacts::create_artifact(
            &self.db,
            cid,
            execution_run_id,
            d,
            st,
            k,
            l,
            c,
            fp,
            ct,
            m,
        )
        .await
    }
    async fn get_artifacts_for_conversation(
        &self,
        cid: i64,
        lim: i64,
    ) -> anyhow::Result<Vec<Artifact>> {
        crate::backend::artifacts::get_artifacts_for_conversation(&self.db, cid, lim).await
    }
    async fn get_artifacts_by_ids(&self, ids: &[i64]) -> anyhow::Result<Vec<Artifact>> {
        crate::backend::artifacts::get_artifacts_by_ids(&self.db, ids).await
    }
    async fn get_artifacts_for_execution_run(&self, rid: i64) -> anyhow::Result<Vec<Artifact>> {
        crate::backend::artifacts::get_artifacts_for_execution_run(&self.db, rid).await
    }
}
