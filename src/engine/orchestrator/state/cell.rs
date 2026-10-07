use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CellState {
    pub cell_id: String,
    pub facts: Vec<String>,
    pub sources: Vec<String>,
    pub unknowns: Vec<String>,
}

impl CellState {
    pub fn new(cell_id: &str) -> Self {
        Self {
            cell_id: cell_id.to_string(),
            ..Default::default()
        }
    }

    pub fn add_fact(&mut self, fact: String) {
        if !self.facts.contains(&fact) {
            self.facts.push(fact);
        }
    }

    pub fn add_source(&mut self, source: String) {
        if !self.sources.contains(&source) {
            self.sources.push(source);
        }
    }

    pub fn to_prompt_string(&self) -> String {
        let mut s = format!("═══ CELL STATE: {} ═══\n", self.cell_id);
        s.push_str("FACTS:\n");
        for f in &self.facts {
            s.push_str(&format!("- {}\n", f));
        }
        s.push_str("\nSOURCES:\n");
        for src in &self.sources {
            s.push_str(&format!("- {}\n", src));
        }
        s
    }
}
