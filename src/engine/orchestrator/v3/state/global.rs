use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GlobalState {
    pub verified_facts: Vec<String>,
    pub authoritative_sources: Vec<String>,
    pub global_insights: Vec<String>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn commit_fact(&mut self, fact: String) {
        if !self.verified_facts.contains(&fact) {
            self.verified_facts.push(fact);
        }
    }

    pub fn commit_source(&mut self, source: String) {
        if !self.authoritative_sources.contains(&source) {
            self.authoritative_sources.push(source);
        }
    }

    pub fn to_prompt_string(&self) -> String {
        let mut s = String::from("═══ GLOBAL LATTICE STATE ═══\n");
        s.push_str("VERIFIED FACTS:\n");
        for f in &self.verified_facts { s.push_str(&format!("- {}\n", f)); }
        s.push_str("\nAUTHORITATIVE SOURCES:\n");
        for src in &self.authoritative_sources { s.push_str(&format!("- {}\n", src)); }
        s
    }
}
