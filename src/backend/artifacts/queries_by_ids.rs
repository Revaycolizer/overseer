use super::queries::Artifact;
use crate::db::Database;

pub async fn get_artifacts_by_ids(
    db: &Database,
    artifact_ids: &[i64],
) -> anyhow::Result<Vec<Artifact>> {
    if artifact_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut rows = Vec::new();
    for artifact_id in artifact_ids {
        if let Some(artifact) = sqlx::query_as::<_, Artifact>(
            "SELECT
                id,
                conversation_id,
                execution_run_id,
                domain,
                step_text,
                kind,
                label,
                content,
                file_path,
                content_type,
                metadata_json,
                created_at
             FROM artifacts
             WHERE id = ?",
        )
        .bind(*artifact_id)
        .fetch_optional(&db.pool)
        .await?
        {
            rows.push(artifact);
        }
    }
    Ok(rows)
}
