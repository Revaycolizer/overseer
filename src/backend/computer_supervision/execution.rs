use crate::db::Database;

pub async fn register_computer_action_execution(
    db: &Database,
    tool_name: &str,
    action_summary: &str,
) -> anyhow::Result<i64> {
    let current = super::super::get_setting_i64(db, "computer_session_action_count", 0).await;
    let next = current + 1;
    super::super::set_setting(db, "computer_session_action_count", &next.to_string()).await?;
    super::session::clear_computer_pending_action(db).await?;
    super::super::set_setting(
        db,
        "computer_session_note",
        &format!(
            "Executed computer action {}.\nTool: {}\nAction: {}",
            next, tool_name, action_summary
        ),
    )
    .await?;
    Ok(next)
}
