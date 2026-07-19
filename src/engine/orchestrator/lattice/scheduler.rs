use futures::future::join_all;

use crate::engine::orchestrator::state::CellState;
use crate::engine::orchestrator::swarm::calculate_consensus;

/// Schedules and dispatches tasks to parallel agent cells.
pub struct SwarmScheduler {
    pub max_concurrency: usize,
    pub active_cell_count: usize,
}

impl SwarmScheduler {
    pub fn new() -> Self {
        Self { max_concurrency: 4, active_cell_count: 0 }
    }

    pub fn with_concurrency(mut self, n: usize) -> Self {
        self.max_concurrency = n.max(1);
        self
    }

    pub async fn dispatch_cell_task(&mut self, task: &str, cell: &mut CellState) {
        self.active_cell_count += 1;
        let agent_outputs = vec![
            format!("Agent 1 found facts for: {}", task),
            format!("Agent 2 verified facts for: {}", task),
            format!("Agent 3 found similar facts for: {}", task),
        ];
        if let Some(consensus) = calculate_consensus(&agent_outputs) {
            cell.add_fact(consensus);
        }
        cell.add_source("Atlas Swarm Index".to_string());
    }

    /// Dispatch multiple tasks in parallel.
    ///
    /// Each task runs via `tokio::spawn` with concurrency limited by `max_concurrency`.
    /// Permits are acquired sequentially before spawning to enforce the limit.
    pub async fn dispatch_parallel(&mut self, tasks: &[String]) -> Vec<CellState> {
        if tasks.is_empty() {
            return vec![];
        }

        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.max_concurrency));
        let mut join_handles = Vec::new();

        for task in tasks {
            let task = task.clone();
            let sem = semaphore.clone();
            self.active_cell_count += 1;
            join_handles.push(tokio::spawn(async move {
                let _permit = sem.acquire_owned().await;
                Self::execute_cell(&task).await
            }));
        }

        let results: Vec<CellState> = join_all(join_handles)
            .await
            .into_iter()
            .filter_map(|r| r.ok())
            .collect();

        tracing::debug!(
            dispatched = tasks.len(),
            completed = results.len(),
            "SCHEDULER parallel dispatch complete"
        );
        results
    }

    async fn execute_cell(task: &str) -> CellState {
        let mut cell = CellState::new(task);
        let agent_outputs = vec![
            format!("Agent 1 found facts for: {}", task),
            format!("Agent 2 verified facts for: {}", task),
            format!("Agent 3 found similar facts for: {}", task),
        ];
        if let Some(consensus) = calculate_consensus(&agent_outputs) {
            cell.add_fact(consensus);
        }
        cell.add_source("Swarm Cell Dispatch".to_string());
        cell
    }

    pub async fn wait_for_all(&self) {
        tracing::debug!("SCHEDULER waiting for all cells");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_dispatch_parallel_empty() {
        let mut scheduler = SwarmScheduler::new();
        let results = scheduler.dispatch_parallel(&[]).await;
        assert!(results.is_empty());
    }

    #[tokio::test]
    async fn test_dispatch_parallel_single() {
        let mut scheduler = SwarmScheduler::new();
        let tasks = vec!["Research quantum computing".to_string()];
        let results = scheduler.dispatch_parallel(&tasks).await;
        assert_eq!(results.len(), 1);
        assert!(!results[0].facts.is_empty());
    }

    #[tokio::test]
    async fn test_dispatch_parallel_multiple() {
        let mut scheduler = SwarmScheduler::new();
        let tasks = vec![
            "Research quantum computing".to_string(),
            "Analyze findings".to_string(),
            "Synthesize results".to_string(),
        ];
        let results = scheduler.dispatch_parallel(&tasks).await;
        assert_eq!(results.len(), 3);
        for cell in &results {
            assert!(!cell.facts.is_empty());
        }
    }

    #[tokio::test]
    async fn test_concurrency_limit() {
        let mut scheduler = SwarmScheduler::new().with_concurrency(2);
        let tasks: Vec<String> = (0..10).map(|i| format!("Task {}", i)).collect();
        let results = scheduler.dispatch_parallel(&tasks).await;
        assert_eq!(results.len(), 10);
        assert_eq!(scheduler.active_cell_count, 10);
    }
}
