use crate::db::Database;

pub async fn stage_computer_approval_request(
    db: &Database,
    domain: &str,
    tool_name: &str,
    action_summary: &str,
) -> anyhow::Result<()> {
    super::super::set_setting(db, "computer_approval_state", "pending").await?;
    super::super::set_setting(db, "computer_pending_tool", tool_name).await?;
    super::super::set_setting(db, "computer_pending_action", action_summary).await?;
    super::super::set_setting(db, "computer_pending_domain", domain).await?;
    super::super::set_setting(
        db,
        "computer_session_note",
        &format!(
            "Approval requested for the next computer action.\nDomain: {}\nTool: {}\nAction: {}",
            domain, tool_name, action_summary
        ),
    )
    .await?;
    Ok(())
}

pub async fn approve_pending_computer_action(db: &Database) -> anyhow::Result<()> {
    let pending_tool = super::super::get_setting_or(db, "computer_pending_tool", "").await;
    let pending_action = super::super::get_setting_or(db, "computer_pending_action", "").await;
    super::super::set_setting(db, "computer_approval_state", "approved_once").await?;
    super::super::set_setting(
        db,
        "computer_session_note",
        &format!(
            "Next approved computer action is armed.\nTool: {}\nAction: {}",
            if pending_tool.trim().is_empty() {
                "(not provided)"
            } else {
                pending_tool.trim()
            },
            if pending_action.trim().is_empty() {
                "(not provided)"
            } else {
                pending_action.trim()
            }
        ),
    )
    .await?;
    Ok(())
}

pub async fn reject_pending_computer_action(db: &Database) -> anyhow::Result<()> {
    super::session::clear_computer_pending_action(db).await?;
    super::super::set_setting(
        db,
        "computer_session_note",
        "Pending computer action was rejected. Overseer should propose a new action before trying again.",
    )
    .await?;
    Ok(())
}
