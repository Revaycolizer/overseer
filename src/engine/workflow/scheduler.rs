use std::str::FromStr;
use std::sync::Arc;
use crate::db::Database;

pub struct WorkflowScheduler {
    db: Arc<Database>,
}

impl WorkflowScheduler {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    pub async fn start(self) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
            loop {
                interval.tick().await;
                self.check_schedules().await;
            }
        });
    }

    async fn check_schedules(&self) {
        let schedules = sqlx::query_as::<_, (i64, String)>(
            "SELECT w.id, n.config_json FROM workflow_nodes n
             JOIN workflows w ON w.id = n.workflow_id
             WHERE n.node_type = 'schedule_trigger'",
        )
        .fetch_all(&self.db.pool)
        .await
        .unwrap_or_default();

        for (workflow_id, config_json) in schedules {
            let cfg: serde_json::Value = serde_json::from_str(&config_json).unwrap_or_default();
            let expr = cfg.get("cron").and_then(|v| v.as_str()).unwrap_or("");
            if expr.is_empty() { continue; }
            if let Ok(schedule) = cron::Schedule::from_str(expr) {
                let now = chrono::Utc::now();
                if schedule.upcoming(chrono::Utc).take(1).any(|t| {
                    let diff = t - now;
                    diff.num_seconds().abs() < 120
                }) {
                    self.execute_workflow(workflow_id).await;
                }
            }
        }
    }

    async fn execute_workflow(&self, workflow_id: i64) {
        let nodes = sqlx::query_as::<_, (i64, String, String)>(
            "SELECT id, node_type, config_json FROM workflow_nodes WHERE workflow_id = ?",
        )
        .bind(workflow_id)
        .fetch_all(&self.db.pool)
        .await
        .unwrap_or_default();

        let graph = crate::engine::workflow::graph::WorkflowGraph {
            nodes: nodes.into_iter().map(|(id, node_type, config_json)| {
                crate::engine::workflow::graph::WorkflowNode { id, workflow_id, node_type, label: String::new(), config_json }
            }).collect(),
            edges: vec![],
        };

        let runner = crate::engine::workflow::runner::WorkflowRunner::new(graph, self.db.clone());
        let _ = runner.execute().await;
    }
}
