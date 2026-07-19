/// Structured Evidence DAG (Directed Acyclic Graph).
///
/// Tracks the provenance of every claim: which agent produced it, what it
/// was derived from, and its verification status. This replaces flat
/// `Vec<String>` fact lists with a verifiable, traversable graph.
use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};

/// Verification status of an evidence node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceStatus {
    /// Newly created, not yet checked.
    Pending,
    /// Cross-referenced and confirmed by another source.
    Verified,
    /// Contradicted by other evidence.
    Contradicted,
    /// Source became unavailable or stale.
    Expired,
}

impl EvidenceStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, EvidenceStatus::Verified | EvidenceStatus::Contradicted | EvidenceStatus::Expired)
    }
}

/// A single node in the evidence DAG.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceNode {
    /// Unique identifier (e.g., "E-<snowflake>").
    pub id: String,
    /// The factual claim.
    pub claim: String,
    /// Which agent or tool produced this.
    pub source: String,
    /// IDs of evidence nodes this was derived from.
    pub parent_ids: Vec<String>,
    /// Confidence score [0.0, 1.0].
    pub confidence: f32,
    /// Verification status.
    pub status: EvidenceStatus,
    /// Human-readable explanation of the status.
    pub status_reason: Option<String>,
    /// When this node was created.
    pub created_at: chrono::NaiveDateTime,
}

impl EvidenceNode {
    pub fn new(id: String, claim: String, source: String) -> Self {
        Self {
            id,
            claim,
            source,
            parent_ids: Vec::new(),
            confidence: 1.0,
            status: EvidenceStatus::Pending,
            status_reason: None,
            created_at: chrono::Utc::now().naive_utc(),
        }
    }

    /// Add a parent edge.
    pub fn with_parent(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_ids.push(parent_id.into());
        self
    }
}

/// The evidence DAG: a directed acyclic graph of evidence nodes.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvidenceDag {
    nodes: HashMap<String, EvidenceNode>,
    /// parent_id -> set of child_ids
    edges: HashMap<String, HashSet<String>>,
}

impl EvidenceDag {
    pub fn new() -> Self {
        Self::default()
    }

    /// Insert a node into the DAG.
    pub fn add_node(&mut self, node: EvidenceNode) {
        let id = node.id.clone();
        // Register parent→child edges
        for parent_id in &node.parent_ids {
            self.edges
                .entry(parent_id.clone())
                .or_default()
                .insert(id.clone());
        }
        self.nodes.insert(id, node);
    }

    /// Get a node by ID.
    pub fn get(&self, id: &str) -> Option<&EvidenceNode> {
        self.nodes.get(id)
    }

    /// Get all nodes.
    pub fn all_nodes(&self) -> Vec<&EvidenceNode> {
        self.nodes.values().collect()
    }

    /// Get all verified nodes.
    pub fn verified_nodes(&self) -> Vec<&EvidenceNode> {
        self.nodes
            .values()
            .filter(|n| n.status == EvidenceStatus::Verified)
            .collect()
    }

    /// Get children of a node (direct descendants).
    pub fn children(&self, id: &str) -> Vec<&EvidenceNode> {
        self.edges
            .get(id)
            .map(|child_ids| {
                child_ids
                    .iter()
                    .filter_map(|cid| self.nodes.get(cid))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get ancestors of a node (recursive parents).
    pub fn ancestors(&self, id: &str) -> Vec<&EvidenceNode> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut stack = vec![id.to_string()];
        while let Some(current) = stack.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            if let Some(node) = self.nodes.get(&current) {
                for parent_id in &node.parent_ids {
                    if !visited.contains(parent_id) {
                        stack.push(parent_id.clone());
                        if let Some(parent) = self.nodes.get(parent_id) {
                            result.push(parent);
                        }
                    }
                }
            }
        }
        result
    }

    /// Total number of nodes.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Convert to a prompt-friendly string.
    pub fn to_prompt_string(&self) -> String {
        let mut s = String::from("═══ EVIDENCE DAG ═══\n");
        let verified = self.verified_nodes();
        s.push_str(&format!("Verified claims: {}\n", verified.len()));
        for node in &verified {
            s.push_str(&format!(
                "  - [{}] {} (source: {}, confidence: {:.2})\n",
                node.id, node.claim, node.source, node.confidence
            ));
        }
        let pending: Vec<&EvidenceNode> = self
            .nodes
            .values()
            .filter(|n| n.status == EvidenceStatus::Pending)
            .collect();
        if !pending.is_empty() {
            s.push_str(&format!("Pending claims: {}\n", pending.len()));
            for node in &pending {
                s.push_str(&format!("  - [{}] {}\n", node.id, node.claim));
            }
        }
        s
    }

    /// Merge another DAG into this one. Nodes from `other` are added;
    /// edges are merged. Duplicate node IDs are skipped (first wins).
    pub fn merge(&mut self, other: EvidenceDag) {
        for (id, node) in other.nodes {
            if !self.nodes.contains_key(&id) {
                self.nodes.insert(id, node);
            }
        }
        for (parent, children) in other.edges {
            let entry = self.edges.entry(parent).or_default();
            for child in children {
                entry.insert(child);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: &str, claim: &str, source: &str) -> EvidenceNode {
        EvidenceNode::new(id.to_string(), claim.to_string(), source.to_string())
    }

    #[test]
    fn test_empty_dag() {
        let dag = EvidenceDag::new();
        assert!(dag.is_empty());
        assert_eq!(dag.len(), 0);
    }

    #[test]
    fn test_add_and_retrieve() {
        let mut dag = EvidenceDag::new();
        let node = make_node("E1", "Quantum computing uses superposition", "web_search");
        dag.add_node(node);
        assert_eq!(dag.len(), 1);
        assert!(dag.get("E1").is_some());
        assert_eq!(dag.get("E1").unwrap().claim, "Quantum computing uses superposition");
    }

    #[test]
    fn test_parent_child_edges() {
        let mut dag = EvidenceDag::new();
        let parent = make_node("E1", "Root claim", "research");
        dag.add_node(parent);
        let child = make_node("E2", "Derived insight", "analysis").with_parent("E1");
        dag.add_node(child);

        let children = dag.children("E1");
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].id, "E2");

        let ancestors = dag.ancestors("E2");
        assert_eq!(ancestors.len(), 1);
        assert_eq!(ancestors[0].id, "E1");
    }

    #[test]
    fn test_verified_filter() {
        let mut dag = EvidenceDag::new();
        let mut v1 = make_node("E1", "Verified claim", "tool_a");
        v1.status = EvidenceStatus::Verified;
        dag.add_node(v1);
        let v2 = make_node("E2", "Pending claim", "tool_b");
        dag.add_node(v2);

        let verified = dag.verified_nodes();
        assert_eq!(verified.len(), 1);
        assert_eq!(verified[0].id, "E1");
    }

    #[test]
    fn test_merge() {
        let mut dag1 = EvidenceDag::new();
        dag1.add_node(make_node("E1", "Claim 1", "source_a"));

        let mut dag2 = EvidenceDag::new();
        dag2.add_node(make_node("E2", "Claim 2", "source_b"));

        dag1.merge(dag2);
        assert_eq!(dag1.len(), 2);
    }

    #[test]
    fn test_ancestors_recursive() {
        let mut dag = EvidenceDag::new();
        dag.add_node(make_node("E1", "Root", "source"));
        dag.add_node(make_node("E2", "Child", "source").with_parent("E1"));
        dag.add_node(make_node("E3", "Grandchild", "source").with_parent("E2"));

        let ancestors = dag.ancestors("E3");
        assert_eq!(ancestors.len(), 2);
        let ancestor_ids: Vec<&str> = ancestors.iter().map(|n| n.id.as_str()).collect();
        assert!(ancestor_ids.contains(&"E1"));
        assert!(ancestor_ids.contains(&"E2"));
    }
}
