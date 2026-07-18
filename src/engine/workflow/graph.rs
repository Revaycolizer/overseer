use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct WorkflowNode {
    pub id: i64,
    pub workflow_id: i64,
    pub node_type: String,
    pub label: String,
    pub config_json: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowEdge {
    pub id: i64,
    pub source_node: i64,
    pub target_node: i64,
    pub source_port: String,
    pub target_port: String,
}

#[derive(Debug, Clone)]
pub struct WorkflowGraph {
    pub nodes: Vec<WorkflowNode>,
    pub edges: Vec<WorkflowEdge>,
}

impl WorkflowGraph {
    pub fn topological_sort(&self) -> Result<Vec<i64>, Vec<i64>> {
        let mut in_degree: HashMap<i64, usize> = self.nodes.iter().map(|n| (n.id, 0)).collect();
        let mut adj: HashMap<i64, Vec<i64>> = self.nodes.iter().map(|n| (n.id, vec![])).collect();

        for e in &self.edges {
            if let Some(deg) = in_degree.get_mut(&e.target_node) { *deg += 1; }
            if let Some(children) = adj.get_mut(&e.source_node) { children.push(e.target_node); }
        }

        let mut queue: VecDeque<i64> = in_degree.iter().filter(|(_, &d)| d == 0).map(|(&id, _)| id).collect();
        let mut sorted = Vec::with_capacity(self.nodes.len());

        while let Some(node) = queue.pop_front() {
            sorted.push(node);
            if let Some(children) = adj.get(&node) {
                for &child in children {
                    if let Some(deg) = in_degree.get_mut(&child) {
                        *deg -= 1;
                        if *deg == 0 { queue.push_back(child); }
                    }
                }
            }
        }

        if sorted.len() == self.nodes.len() { Ok(sorted) } else { Err(sorted) }
    }
}
