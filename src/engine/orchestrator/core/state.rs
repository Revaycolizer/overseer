use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvidenceChunk {
    pub id: String, // e.g., "E1", "E2"
    pub snowflake_id: i64,
    pub text: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StructuredState {
    pub evidence: Vec<EvidenceChunk>,
    pub derived_insights: Vec<String>,
    pub unknowns: Vec<String>,
    pub next_id: usize,
}

impl StructuredState {
    pub fn new() -> Self {
        Self {
            evidence: Vec::new(),
            derived_insights: Vec::new(),
            unknowns: Vec::new(),
            next_id: 1,
        }
    }

    pub fn to_prompt_string(&self) -> String {
        let mut out = String::from("═══ STRUCTURED_STATE (EVIDENCE-FIRST) ═══\n");
        out.push_str("GROUNDED EVIDENCE:\n");
        for e in &self.evidence { 
            out.push_str(&format!("[{}] {} (Source: {}) [ID: {}]\n", e.id, e.text, e.source, e.snowflake_id)); 
        }
        out.push_str("\nDERIVED_INSIGHTS:\n");
        for i in &self.derived_insights { out.push_str(&format!("- {}\n", i)); }
        out.push_str("\nUNKNOWNS / GAPS:\n");
        for u in &self.unknowns { out.push_str(&format!("- {}\n", u)); }
        out.push_str("══════════════════════");
        out
    }

    pub fn get_known_urls(&self) -> std::collections::HashSet<String> {
        self.evidence.iter().map(|e| e.source.clone()).filter(|s| s.starts_with("http")).collect()
    }
}
