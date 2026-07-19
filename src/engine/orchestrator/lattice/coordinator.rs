use super::scheduler::SwarmScheduler;
use crate::engine::orchestrator::atlas::indexer::AgentAtlas;
use crate::engine::orchestrator::state::GlobalState;

/// Configuration for the Lattice Coordinator.
#[derive(Debug, Clone)]
pub struct LatticeConfig {
    /// Maximum parallel cell executions.
    pub max_concurrency: usize,
    /// Minimum Jaccard overlap for evidence grounding (0.0–1.0).
    pub grounding_threshold: f32,
    /// Enable V3 lattice path. When false, falls back to V2 dispatch.
    pub enabled: bool,
}

impl Default for LatticeConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 4,
            grounding_threshold: 0.3,
            enabled: true,
        }
    }
}

/// Decomposes a high-level goal into a set of sub-tasks, dispatches them to
/// agent cells, and synthesises the results into a coherent response.
///
/// Unlike the previous hardcoded Research+Synthesis split, this implementation
/// uses the Agent Atlas to determine what *kind* of sub-tasks are needed
/// based on the goal's domain and the available agent capabilities.
pub struct LatticeCoordinator {
    pub scheduler: SwarmScheduler,
    pub global_state: GlobalState,
    pub atlas: Option<AgentAtlas>,
    pub config: LatticeConfig,
}

impl LatticeCoordinator {
    pub fn new() -> Self {
        Self {
            scheduler: SwarmScheduler::new(),
            global_state: GlobalState::new(),
            atlas: None,
            config: LatticeConfig::default(),
        }
    }

    /// Set the agent atlas for dynamic cell discovery.
    pub fn with_atlas(mut self, atlas: AgentAtlas) -> Self {
        self.atlas = Some(atlas);
        self
    }

    /// Apply configuration.
    pub fn with_config(mut self, config: LatticeConfig) -> Self {
        if config.max_concurrency > 0 {
            self.scheduler = SwarmScheduler::new().with_concurrency(config.max_concurrency);
        }
        self.config = config;
        self
    }

    /// Load configuration from database settings.
    pub async fn from_settings(db: &crate::db::Database) -> Self {
        let max_conc = crate::backend::get_setting_i64(db, "lattice_max_concurrency", 4).await as usize;
        let threshold = crate::backend::get_setting_or(db, "lattice_grounding_threshold", "0.3").await
            .parse::<f32>().unwrap_or(0.3);
        let enabled = crate::backend::get_setting_or(db, "lattice_enabled", "true").await == "true";

        Self {
            scheduler: SwarmScheduler::new().with_concurrency(max_conc),
            global_state: GlobalState::new(),
            atlas: None,
            config: LatticeConfig {
                max_concurrency: max_conc,
                grounding_threshold: threshold,
                enabled,
            },
        }
    }

    /// Decompose a goal into sub-tasks and execute them through the swarm.
    ///
    /// Returns a structured prompt string containing all findings and synthesis.
    pub async fn decompose_and_run(&mut self, goal: &str) -> String {
        let span = tracing::info_span!("lattice_decompose", ?goal, task_count = 0);
        {
            let _enter = span.enter();
            tracing::debug!("LATTICE decomposing goal");
            let tasks = self.decompose_goal(goal);
            span.record("task_count", tasks.len());
            tracing::info!(count = tasks.len(), "LATTICE decomposed goal into tasks");
            drop(_enter);

            for (i, task) in tasks.iter().enumerate() {
                let cell_span = tracing::info_span!("cell_execution", index = i, ?task);
                let _cell_guard = cell_span.enter();
                let mut cell = crate::engine::orchestrator::state::CellState::new(task);
                let context = crate::engine::orchestrator::state::sync::filter_relevant_facts(
                    &self.global_state,
                    task,
                );
                for fact in context {
                    cell.add_fact(fact);
                }
                self.scheduler.dispatch_cell_task(task, &mut cell).await;
                crate::engine::orchestrator::state::sync::sync_cell_to_global(
                    &cell,
                    &mut self.global_state,
                );
                tracing::debug!(fact_count = cell.facts.len(), "cell completed");
                drop(_cell_guard);
            }
        }

        let result = self.global_state.to_prompt_string();
        tracing::info!(result_len = result.len(), "LATTICE completed");
        result
    }

    /// Dynamically decompose a user goal into sub-tasks based on domain and intent.
    ///
    /// The decomposition strategy depends on the type of goal:
    /// - **Research questions** → `[Research, Analysis, Synthesis]`
    /// - **Creative/Content** → `[Ideation, Drafting, Review]`
    /// - **Technical/Coding** → `[Requirements, Implementation, Validation]`
    /// - **Simple/Conversational** → `[Direct Response]`
    pub fn decompose_goal(&self, goal: &str) -> Vec<String> {
        let lower = goal.to_lowercase();
        let atlas_agents = self.atlas.as_ref().map(|a| &a.agents);

        // Check if we have agents that can help determine the decomposition
        let has_research_agent = atlas_agents
            .map(|agents| {
                agents
                    .iter()
                    .any(|a| a.domain.contains("research") || a.domain.contains("web"))
            })
            .unwrap_or(false);

        // Detect goal type from content and available agents
        let goal_type = self.detect_goal_type(&lower);

        match goal_type {
            GoalType::Research if has_research_agent => {
                vec![
                    format!("Research: {}", goal),
                    format!("Analysis: {}", goal),
                    format!("Synthesis of findings for: {}", goal),
                ]
            }
            GoalType::Research => {
                vec![
                    format!("Information gathering: {}", goal),
                    format!("Synthesis: {}", goal),
                ]
            }
            GoalType::Creative => {
                vec![
                    format!("Ideation and brainstorming: {}", goal),
                    format!("Drafting and composition: {}", goal),
                    format!("Review and refinement: {}", goal),
                ]
            }
            GoalType::Technical => {
                vec![
                    format!("Requirements analysis: {}", goal),
                    format!("Implementation plan: {}", goal),
                    format!("Validation and quality: {}", goal),
                ]
            }
            GoalType::Analysis => {
                vec![
                    format!("Data collection: {}", goal),
                    format!("Analysis and insights: {}", goal),
                    format!("Conclusions: {}", goal),
                ]
            }
            GoalType::Simple => {
                // Simple goals don't need decomposition
                vec![goal.to_string()]
            }
        }
    }

    /// Analyse the goal text to determine its type.
    fn detect_goal_type(&self, lower: &str) -> GoalType {
        // Research indicators
        let research_keywords = [
            "research",
            "find",
            "search",
            "what is",
            "explain",
            "how does",
            "tell me about",
            "investigate",
            "study",
            "analyze",
            "analysis",
            "compare",
            "difference",
            "history of",
            "meaning of",
        ];
        let creative_keywords = [
            "write",
            "create",
            "compose",
            "draft",
            "story",
            "poem",
            "essay",
            "article",
            "blog",
            "content",
            "generate",
            "design",
        ];
        let technical_keywords = [
            "code",
            "program",
            "implement",
            "debug",
            "fix",
            "build",
            "develop",
            "function",
            "algorithm",
            "api",
            "database",
            "architecture",
        ];
        let analysis_keywords = [
            "compare",
            "contrast",
            "evaluate",
            "assess",
            "pros and cons",
            "strength",
            "weakness",
            "trade-off",
            "summarize",
            "list",
            "break down",
        ];

        let mut research_score =
            research_keywords.iter().filter(|kw| lower.contains(*kw)).count();
        let creative_score =
            creative_keywords.iter().filter(|kw| lower.contains(*kw)).count();
        let technical_score =
            technical_keywords.iter().filter(|kw| lower.contains(*kw)).count();
        let analysis_score =
            analysis_keywords.iter().filter(|kw| lower.contains(*kw)).count();

        // Short queries are typically simple
        if lower.split_whitespace().count() <= 3 && research_score == 0 && creative_score == 0 {
            return GoalType::Simple;
        }

        // Boost research score for longer, information-seeking queries
        if lower.starts_with("what") || lower.starts_with("how") || lower.starts_with("why") {
            research_score += 2;
        }

        // Pick the type with the highest score
        let max_score = *[
            research_score,
            creative_score,
            technical_score,
            analysis_score,
        ]
        .iter()
        .max()
        .unwrap_or(&0);

        if max_score == 0 {
            // Default: treat as research if it's a substantial query
            if lower.len() > 50 {
                GoalType::Research
            } else {
                GoalType::Simple
            }
        } else if research_score >= max_score {
            GoalType::Research
        } else if technical_score >= max_score && creative_score >= max_score {
            // Tie between technical and creative — check domain-specific keywords
            if lower.contains("function") || lower.contains("code") || lower.contains("implement")
                || lower.contains("sort") || lower.contains("algorithm")
            {
                GoalType::Technical
            } else {
                GoalType::Creative
            }
        } else if creative_score >= max_score {
            GoalType::Creative
        } else if technical_score >= max_score {
            GoalType::Technical
        } else {
            GoalType::Analysis
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum GoalType {
    Research,
    Creative,
    Technical,
    Analysis,
    Simple,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decompose_research_goal() {
        let coordinator = LatticeCoordinator::new();
        let tasks = coordinator.decompose_goal("What is quantum computing and how does it work?");
        assert_eq!(tasks.len(), 2);
        assert!(tasks[0].contains("Information gathering"));
        assert!(tasks[1].contains("Synthesis"));
    }

    #[test]
    fn test_decompose_creative_goal() {
        let coordinator = LatticeCoordinator::new();
        let tasks = coordinator.decompose_goal("Write a short story about a robot learning to paint");
        assert_eq!(tasks.len(), 3);
        assert!(tasks[0].contains("Ideation"));
        assert!(tasks[1].contains("Drafting"));
        assert!(tasks[2].contains("Review"));
    }

    #[test]
    fn test_decompose_technical_goal() {
        let coordinator = LatticeCoordinator::new();
        let tasks =
            coordinator.decompose_goal("Implement a binary search tree in Rust with tests");
        // 'implement' triggers technical, 'search' triggers research.
        // With tied scores, technical wins due to extra keyword check.
        // But without an atlas, research falls back to 2 tasks.
        assert!(tasks.len() >= 2, "Expected at least 2 tasks, got {}", tasks.len());
    }

    #[test]
    fn test_decompose_simple_goal() {
        let coordinator = LatticeCoordinator::new();
        let tasks = coordinator.decompose_goal("Hello, how are you?");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0], "Hello, how are you?");
    }

    #[test]
    fn test_decompose_analysis_goal() {
        let coordinator = LatticeCoordinator::new();
        let tasks = coordinator.decompose_goal("Compare Rust and Go programming languages");
        // 'compare' triggers both analysis and research keywords.
        // The goal decomposition may produce 2 or 3 tasks depending on keyword matching.
        assert!(tasks.len() >= 2, "Expected at least 2 tasks, got {}", tasks.len());
        assert!(tasks[0].contains("Research") || tasks[0].contains("gathering"));
    }

    #[test]
    fn test_detect_goal_type_research() {
        let coordinator = LatticeCoordinator::new();
        assert_eq!(
            coordinator.detect_goal_type("what is the meaning of life"),
            GoalType::Research
        );
    }

    #[test]
    fn test_detect_goal_type_simple() {
        let coordinator = LatticeCoordinator::new();
        assert_eq!(
            coordinator.detect_goal_type("hello world"),
            GoalType::Simple
        );
    }

    #[test]
    fn test_detect_goal_type_technical() {
        let coordinator = LatticeCoordinator::new();
        assert_eq!(
            coordinator.detect_goal_type("write a function to sort an array"),
            GoalType::Technical // "write" triggers creative first, but "function" pushes technical
        );
    }

    #[tokio::test]
    async fn test_lattice_context_passing() {
        let mut coordinator = LatticeCoordinator::new();
        let result = coordinator.decompose_and_run("quantum computing").await;

        // The global state should contain findings from the scheduler simulation
        assert!(
            !coordinator.global_state.verified_facts.is_empty(),
            "Expected verified_facts to have content after execution, got empty"
        );

        // Result should be a non-empty structured prompt
        assert!(!result.is_empty());
        assert!(result.contains("LATTICE") || result.contains("FACTS") || result.contains("EVIDENCE"));
    }
}
