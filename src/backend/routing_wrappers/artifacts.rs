use crate::db::Database;
use crate::backend::Artifact;
use crate::backend::client::ArtifactClient;

pub async fn create_artifact(db: &Database, conversation_id: i64, execution_run_id: Option<i64>, domain: Option<&str>, step_text: Option<&str>, kind: &str, label: &str, content: Option<&str>, file_path: Option<&str>, content_type: Option<&str>, metadata_json: Option<&str>) -> anyhow::Result<i64> {
    if db.is_remote() { return db.remote_client().create_artifact(conversation_id, execution_run_id, domain, step_text, kind, label, content, file_path, content_type, metadata_json).await; }
    crate::backend::artifacts::create_artifact(db, conversation_id, execution_run_id, domain, step_text, kind, label, content, file_path, content_type, metadata_json).await
}

pub async fn get_artifacts_for_conversation(db: &Database, conversation_id: i64, limit: i64) -> anyhow::Result<Vec<Artifact>> {
    if db.is_remote() { return db.remote_client().get_artifacts_for_conversation(conversation_id, limit).await; }
    crate::backend::artifacts::get_artifacts_for_conversation(db, conversation_id, limit).await
}

pub async fn get_artifacts_by_ids(db: &Database, artifact_ids: &[i64]) -> anyhow::Result<Vec<Artifact>> {
    if db.is_remote() { return db.remote_client().get_artifacts_by_ids(artifact_ids).await; }
    crate::backend::artifacts::get_artifacts_by_ids(db, artifact_ids).await
}

pub async fn get_artifacts_for_execution_run(db: &Database, execution_run_id: i64) -> anyhow::Result<Vec<Artifact>> {
    if db.is_remote() { return db.remote_client().get_artifacts_for_execution_run(execution_run_id).await; }
    crate::backend::artifacts::get_artifacts_for_execution_run(db, execution_run_id).await
}
