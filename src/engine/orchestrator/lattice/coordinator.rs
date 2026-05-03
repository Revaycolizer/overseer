use super::scheduler::SwarmScheduler;
use crate::engine::orchestrator::state::GlobalState;

pub struct LatticeCoordinator {
    pub scheduler: SwarmScheduler,
    pub global_state: GlobalState,
}

impl LatticeCoordinator {
    pub fn new() -> Self {
        Self {
            scheduler: SwarmScheduler::new(),
            global_state: GlobalState::new(),
        }
    }

    pub async fn decompose_and_run(&mut self, goal: &str) -> String {
        println!("LATTICE: Decomposing goal into swarm tasks: {}", goal);
        
        // V3.1 logic: Identify required cells and dispatch.
        // For Phase 1, we decompose into Research and Synthesis steps for context testing.
        let tasks = vec!["Research: ".to_string() + goal, "Synthesis: ".to_string() + goal];
        
        for task in tasks {
            // 1. Context Sharding: Filter relevant global facts for this cell
            let mut cell = crate::engine::orchestrator::state::CellState::new(&task);
            let context = crate::engine::orchestrator::state::sync::filter_relevant_facts(&self.global_state, &task);
            for fact in context { cell.add_fact(fact); }
            
            // 2. Swarm Execution: Dispatch to cell and wait for consensus
            self.scheduler.dispatch_cell_task(&task, &mut cell).await;
            
            // 3. State Synchronization: Commit cell findings to Global Lattice
            crate::engine::orchestrator::state::sync::sync_cell_to_global(&cell, &mut self.global_state);
        }

        self.global_state.to_prompt_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lattice_context_passing() {
        let mut coordinator = LatticeCoordinator::new();
        let result = coordinator.decompose_and_run("quantum computing").await;
        
        // Verify that findings from the first task (Research) exist in Global State
        assert!(coordinator.global_state.verified_facts.iter().any(|f| f.contains("Agent 1 found facts for: Research: quantum computing")));
        // Verify that the second task (Synthesis) results are also there
        assert!(coordinator.global_state.verified_facts.iter().any(|f| f.contains("Agent 1 found facts for: Synthesis: quantum computing")));
        
        println!("Test result:\n{}", result);
    }
}
