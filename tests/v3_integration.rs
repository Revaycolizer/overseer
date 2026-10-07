//! V3 Architecture integration tests.
//!
//! Covers atlas (search, grouping), swarm (consensus, verification),
//! lattice (decomposition, scheduling, compilation), and state (DAG).
//! These are pure-logic tests that don't need a database or Ollama.

use std::collections::HashMap;

use overseer::engine::orchestrator::atlas::{group_into_cells, search_atlas, AgentAtlas};
use overseer::engine::orchestrator::lattice::coordinator::LatticeCoordinator;
use overseer::engine::orchestrator::lattice::scheduler::SwarmScheduler;
use overseer::engine::orchestrator::state::{CellState, EvidenceDag, EvidenceNode, EvidenceStatus};
use overseer::engine::orchestrator::swarm::{calculate_consensus, SwarmVerifier};
use overseer::engine::utils::discovery::AgentInfo;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_agent(id: i64, name: &str, domain: &str, desc: &str) -> AgentInfo {
    AgentInfo {
        id,
        name: name.into(),
        description: desc.into(),
        tools: vec![],
        instructions: String::new(),
        model: String::new(),
        domain: domain.into(),
        version: "1.0".into(),
        code: None,
        source_files: vec![],
        response_format: "json".into(),
    }
}

fn make_atlas(agents: Vec<AgentInfo>) -> AgentAtlas {
    AgentAtlas::new(agents)
}

// ===========================================================================
// Atlas — Search
// ===========================================================================

#[test]
fn test_atlas_search_tfidf_scoring() {
    // Agents with different domain/description relevance
    let atlas = make_atlas(vec![
        make_agent(
            1,
            "web_search",
            "web",
            "Searches the internet for information",
        ),
        make_agent(2, "file_manager", "files", "Manages local file operations"),
        make_agent(
            3,
            "code_runner",
            "code",
            "Executes and compiles source code",
        ),
    ]);
    // Search for code-related terms — should rank code_runner highest
    let results = search_atlas(&atlas, "compile and run source code", 3);
    assert!(!results.is_empty(), "should return results");
    assert_eq!(
        results[0].name, "code_runner",
        "code_runner should be first"
    );
}

#[test]
fn test_atlas_search_empty_atlas_returns_empty() {
    let atlas = make_atlas(vec![]);
    let results = search_atlas(&atlas, "anything", 5);
    assert!(results.is_empty());
}

#[test]
fn test_atlas_search_returns_all_when_limit_exceeds_count() {
    let atlas = make_atlas(vec![
        make_agent(1, "a", "web", "Web search"),
        make_agent(2, "b", "files", "File ops"),
    ]);
    let results = search_atlas(&atlas, "search", 10);
    assert_eq!(results.len(), 2);
}

#[test]
fn test_atlas_search_deduplicates_results() {
    let atlas = make_atlas(vec![
        make_agent(
            1,
            "web1",
            "web",
            "Web search tool for finding information on the internet",
        ),
        make_agent(
            2,
            "web2",
            "web",
            "Another web search tool for internet information",
        ),
    ]);
    let results = search_atlas(&atlas, "web search tool", 5);
    // Both should appear, but no duplicates
    assert_eq!(results.len(), 2);
    assert_ne!(results[0].id, results[1].id);
}

// ===========================================================================
// Atlas — Grouping
// ===========================================================================

#[test]
fn test_group_into_cells_single_domain() {
    let agents = vec![
        make_agent(1, "a", "web", "Web search"),
        make_agent(2, "b", "web", "Web browse"),
    ];
    let cells = group_into_cells(&agents);
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].len(), 2);
}

#[test]
fn test_group_into_cells_multiple_domains() {
    let agents = vec![
        make_agent(1, "a", "web", "Web search"),
        make_agent(2, "b", "files", "File read"),
        make_agent(3, "c", "chat", "Chat assistant"),
    ];
    let cells = group_into_cells(&agents);
    assert!(
        cells.len() >= 3,
        "expected >=3 cells for 3 domains, got {}",
        cells.len()
    );
}

#[test]
fn test_group_into_cells_large_group_splits() {
    let agents: Vec<AgentInfo> = (0..10)
        .map(|i| {
            make_agent(
                i,
                &format!("agent_{i}"),
                "web",
                "A web-related agent tool for searching",
            )
        })
        .collect();
    let cells = group_into_cells(&agents);
    assert!(
        cells.len() > 1,
        "large group should split into multiple cells"
    );
}

// ===========================================================================
// Swarm — Consensus
// ===========================================================================

#[test]
fn test_consensus_two_identical_one_different() {
    let outputs = vec![
        "Quantum computing uses superposition and entanglement.".to_string(),
        "Quantum computing uses superposition and entanglement.".to_string(),
        "The weather is sunny today.".to_string(),
    ];
    let result = calculate_consensus(&outputs).unwrap();
    assert!(
        result.to_lowercase().contains("quantum"),
        "consensus should prefer majority topic, got: {}",
        result
    );
}

#[test]
fn test_consensus_tie_breaker() {
    let outputs = vec![
        "Agent A report about machine learning algorithms.".to_string(),
        "Agent B report about deep neural networks.".to_string(),
        "Agent C unrelated random text.".to_string(),
    ];
    let result = calculate_consensus(&outputs);
    assert!(result.is_some(), "tie should still produce a result");
}

#[test]
fn test_consensus_all_different() {
    let outputs = vec![
        "First topic entirely unique words here one.".to_string(),
        "Second topic completely different words here two.".to_string(),
        "Third topic nothing in common words here three.".to_string(),
    ];
    let result = calculate_consensus(&outputs);
    assert!(
        result.is_some(),
        "even all-different outputs should return one"
    );
}

#[test]
fn test_consensus_single_element() {
    let outputs = vec!["Only one agent response.".to_string()];
    let result = calculate_consensus(&outputs);
    assert_eq!(result, Some("Only one agent response.".to_string()));
}

// ===========================================================================
// Swarm — Verifier
// ===========================================================================

fn cell_with_facts(facts: Vec<&str>) -> CellState {
    let mut cell = CellState::new("test_cell");
    for f in facts {
        cell.add_fact(f.to_string());
    }
    cell
}

#[test]
fn test_verifier_high_confidence_match() {
    let cell = cell_with_facts(vec!["Quantum computing uses superposition of states"]);
    let verifier = SwarmVerifier::new(0.3);
    let score = verifier.verify_claims(
        "Quantum computing uses superposition for computation.",
        &cell,
    );
    assert!(
        score > 0.5,
        "expected high score for matching content, got {score}"
    );
}

#[test]
fn test_verifier_no_match_score_low() {
    let cell = cell_with_facts(vec!["Python is a programming language."]);
    let verifier = SwarmVerifier::new(0.5);
    let score = verifier.verify_claims("Mount Everest is the tallest mountain.", &cell);
    assert!(
        score < 0.5,
        "expected low score for unrelated content, got {score}"
    );
}

#[test]
fn test_verifier_empty_cell_returns_high_score() {
    let cell = cell_with_facts(vec![]);
    let verifier = SwarmVerifier::new(0.5);
    let score = verifier.verify_claims("Some random text here.", &cell);
    // Empty cell has no evidence — nothing can be grounded
    assert_eq!(score, 0.0, "empty cell should return 0, got {score}");
}

#[test]
fn test_verifier_empty_sentences_vacuously_true() {
    let cell = cell_with_facts(vec!["Some facts"]);
    let verifier = SwarmVerifier::new(0.5);
    let score = verifier.verify_claims("", &cell);
    assert!((score - 1.0).abs() < 1e-6);
}

// ===========================================================================
// Lattice — Coordinator Decomposition
// ===========================================================================

#[test]
fn test_coordinator_research_decomposition() {
    let coord = LatticeCoordinator::new();
    let tasks = coord.decompose_goal("What is quantum computing and how does entanglement work?");
    assert!(
        tasks.len() >= 2,
        "research goal should decompose into >=2 tasks"
    );
    assert!(
        tasks[0].contains("Information gathering") || tasks[0].contains("Research"),
        "first task should be research-like: {}",
        tasks[0]
    );
}

#[test]
fn test_coordinator_creative_decomposition() {
    let coord = LatticeCoordinator::new();
    let tasks = coord.decompose_goal("Write a short story about a robot learning empathy");
    assert_eq!(tasks.len(), 3, "creative goal should produce 3 tasks");
    assert!(
        tasks[0].contains("Ideation"),
        "first creative task should be ideation"
    );
    assert!(tasks[1].contains("Drafting"), "second should be drafting");
}

#[test]
fn test_coordinator_simple_decomposition_is_singleton() {
    let coord = LatticeCoordinator::new();
    let tasks = coord.decompose_goal("Hello, how are you?");
    assert_eq!(tasks.len(), 1, "simple query should not decompose");
    assert_eq!(tasks[0], "Hello, how are you?");
}

// ===========================================================================
// Lattice — Scheduler
// ===========================================================================

#[tokio::test]
async fn test_scheduler_parallel_respects_concurrency() {
    let mut scheduler = SwarmScheduler::new().with_concurrency(2);
    let tasks: Vec<String> = (0..5).map(|i| format!("Task {}", i)).collect();
    let results = scheduler.dispatch_parallel(&tasks).await;
    assert_eq!(results.len(), 5, "all 5 tasks should complete");
    for cell in &results {
        assert!(!cell.facts.is_empty(), "each cell should have facts");
    }
}

#[tokio::test]
async fn test_scheduler_parallel_no_tasks() {
    let mut scheduler = SwarmScheduler::new();
    let results = scheduler.dispatch_parallel(&[]).await;
    assert!(results.is_empty());
}

// ===========================================================================
// State — Evidence DAG
// ===========================================================================

fn make_node(id: &str, claim: &str) -> EvidenceNode {
    EvidenceNode::new(id.to_string(), claim.to_string(), "test_source".to_string())
}

#[test]
fn test_dag_chain_of_evidence() {
    let mut dag = EvidenceDag::new();
    dag.add_node(make_node("E1", "Root claim"));
    dag.add_node(make_node("E2", "Derived claim").with_parent("E1"));
    dag.add_node(make_node("E3", "Final conclusion").with_parent("E2"));

    assert_eq!(dag.len(), 3);
    let ancestors = dag.ancestors("E3");
    assert_eq!(ancestors.len(), 2);
    let ids: Vec<&str> = ancestors.iter().map(|n| n.id.as_str()).collect();
    assert!(ids.contains(&"E1"));
    assert!(ids.contains(&"E2"));
}

#[test]
fn test_dag_merge_preserves_all_nodes() {
    let mut dag1 = EvidenceDag::new();
    dag1.add_node(make_node("E1", "Claim A"));
    let mut dag2 = EvidenceDag::new();
    dag2.add_node(make_node("E2", "Claim B"));
    dag2.add_node(make_node("E3", "Claim C"));

    dag1.merge(dag2);
    assert_eq!(dag1.len(), 3);
}

#[test]
fn test_dag_verified_filter_excludes_pending() {
    let mut dag = EvidenceDag::new();
    let mut v = make_node("E1", "Verified");
    v.status = EvidenceStatus::Verified;
    dag.add_node(v);
    dag.add_node(make_node("E2", "Pending"));

    let verified = dag.verified_nodes();
    assert_eq!(verified.len(), 1);
    assert_eq!(verified[0].id, "E1");
}

#[test]
fn test_dag_empty_verified_returns_empty() {
    let dag = EvidenceDag::new();
    assert!(dag.verified_nodes().is_empty());
}

#[test]
fn test_dag_children_of_unknown_id() {
    let dag = EvidenceDag::new();
    assert!(dag.children("nonexistent").is_empty());
}

// ===========================================================================
// State — CellState
// ===========================================================================

#[test]
fn test_cell_state_deduplicates_facts() {
    let mut cell = CellState::new("test");
    cell.add_fact("Same fact".to_string());
    cell.add_fact("Same fact".to_string());
    assert_eq!(
        cell.facts.len(),
        1,
        "duplicate facts should be deduplicated"
    );
}

#[test]
fn test_cell_state_tracks_sources() {
    let mut cell = CellState::new("test");
    cell.add_source("Source A".to_string());
    cell.add_source("Source B".to_string());
    assert_eq!(cell.sources.len(), 2);
}

#[test]
fn test_cell_state_prompt_includes_id() {
    let cell = CellState::new("custom-cell-id");
    let prompt = cell.to_prompt_string();
    assert!(
        prompt.contains("custom-cell-id"),
        "prompt should include cell id"
    );
}
