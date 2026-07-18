use crate::db::Database;

pub async fn activate_computer_session(
    db: &Database,
    session_state: &str,
    note: &str,
) -> anyhow::Result<()> {
    super::super::set_setting(db, "computer_session_state", session_state).await?;
    super::super::set_setting(db, "computer_session_note", note).await?;
    if session_state == "idle" {
        super::super::set_setting(db, "computer_session_started_at", "").await?;
        super::super::set_setting(db, "computer_session_action_count", "0").await?;
    } else {
        let started_at = chrono::Utc::now().to_rfc3339();
        super::super::set_setting(db, "computer_session_started_at", &started_at).await?;
        super::super::set_setting(db, "computer_session_action_count", "0").await?;
    }
    clear_computer_pending_action(db).await?;
    Ok(())
}

pub async fn clear_computer_pending_action(db: &Database) -> anyhow::Result<()> {
    super::super::set_setting(db, "computer_approval_state", "idle").await?;
    super::super::set_setting(db, "computer_pending_tool", "").await?;
    super::super::set_setting(db, "computer_pending_action", "").await?;
    super::super::set_setting(db, "computer_pending_domain", "").await?;
    Ok(())
}
