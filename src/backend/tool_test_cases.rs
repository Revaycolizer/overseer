use crate::db::Database;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct ToolTestCase {
    pub id: i64,
    pub tool_id: i64,
    pub name: String,
    pub input: String,
    pub expected_substring: Option<String>,
    pub last_status: Option<String>,
    pub last_output: Option<String>,
    pub last_run_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub async fn get_tool_test_cases(db: &Database, tool_id: i64) -> anyhow::Result<Vec<ToolTestCase>> {
    let cases = sqlx::query_as::<_, ToolTestCase>(
        "SELECT id, tool_id, name, input, expected_substring, last_status, last_output, last_run_at, created_at, updated_at
         FROM tool_test_cases
         WHERE tool_id = ?
         ORDER BY updated_at DESC, id DESC",
    )
    .bind(tool_id)
    .fetch_all(&db.pool)
    .await?;

    Ok(cases)
}

pub async fn add_tool_test_case(
    db: &Database,
    tool_id: i64,
    name: &str,
    input: &str,
    expected_substring: Option<&str>,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO tool_test_cases (tool_id, name, input, expected_substring)
         VALUES (?, ?, ?, ?)",
    )
    .bind(tool_id)
    .bind(name)
    .bind(input)
    .bind(expected_substring)
    .execute(&db.pool)
    .await?;

    Ok(())
}

pub async fn update_tool_test_case(db: &Database, test_case: ToolTestCase) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE tool_test_cases
         SET name = ?, input = ?, expected_substring = ?, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(test_case.name)
    .bind(test_case.input)
    .bind(test_case.expected_substring)
    .bind(test_case.id)
    .execute(&db.pool)
    .await?;

    Ok(())
}

pub async fn delete_tool_test_case(db: &Database, case_id: i64) -> anyhow::Result<()> {
    sqlx::query("DELETE FROM tool_test_cases WHERE id = ?")
        .bind(case_id)
        .execute(&db.pool)
        .await?;

    Ok(())
}

pub async fn record_tool_test_case_result(
    db: &Database,
    case_id: i64,
    status: &str,
    output: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE tool_test_cases
         SET last_status = ?, last_output = ?, last_run_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP
         WHERE id = ?",
    )
    .bind(status)
    .bind(output)
    .bind(case_id)
    .execute(&db.pool)
    .await?;

    Ok(())
}
