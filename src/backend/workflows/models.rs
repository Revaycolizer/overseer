use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct Workflow {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub trigger_type: String,
    pub trigger_config: Option<String>,
    pub status: String,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct WorkflowStep {
    pub id: i64,
    pub workflow_id: i64,
    pub step_order: i64,
    pub name: String,
    pub prompt: String,
    pub enabled: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct WorkflowRun {
    pub id: i64,
    pub workflow_id: i64,
    pub conversation_id: Option<i64>,
    pub status: String,
    pub summary: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub finished_at: Option<chrono::NaiveDateTime>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow, PartialEq)]
pub struct WorkflowStepRun {
    pub id: i64,
    pub workflow_run_id: i64,
    pub workflow_step_id: i64,
    pub execution_run_id: Option<i64>,
    pub step_order: i64,
    pub status: String,
    pub output: Option<String>,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub finished_at: Option<chrono::NaiveDateTime>,
}
