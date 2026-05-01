use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvidenceChunk {
    pub id: String, // e.g., "E1", "E2"
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
            out.push_str(&format!("[{}] {} (Source: {})\n", e.id, e.text, e.source)); 
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

    pub fn preview_update(
        &self,
        findings: &std::collections::HashMap<String, serde_json::Value>,
        summary: &str,
        evidence_raw: &[String]
    ) -> Self {
        let mut cloned = self.clone();
        cloned.update_from_findings(findings, summary, evidence_raw);
        cloned
    }

    pub fn update_from_findings(
        &mut self, 
        findings: &std::collections::HashMap<String, serde_json::Value>, 
        summary: &str, 
        evidence_raw: &[String]
    ) {
        // 1. Process Raw Evidence into Structured Chunks
        for e in evidence_raw {
            let mut text = e.clone();
            let mut source = "collected_data".to_string();

            // Try to parse if it's JSON evidence
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(e) {
                if let Some(snippet) = v.get("snippet").or(v.get("text")).and_then(|v| v.as_str()) {
                    text = snippet.to_string();
                }
                if let Some(src) = v.get("url").or(v.get("source")).and_then(|v| v.as_str()) {
                    source = src.to_string();
                }
            }

            // Check if this chunk already exists to avoid duplicates
            if !self.evidence.iter().any(|c| c.text == text && c.source == source) {
                let id = format!("E{}", self.next_id);
                self.evidence.push(EvidenceChunk {
                    id,
                    text,
                    source,
                });
                self.next_id += 1;
            }
        }

        // 2. Accumulate findings into insights
        for (key, val) in findings {
            if let Some(s) = val.as_str() {
                let clean_s = crate::engine::orchestrator::utils::format_helpers::strip_grounding_markers(s);
                let insight = format!("{}: {}", key, clean_s);
                if !self.derived_insights.contains(&insight) {
                    self.derived_insights.push(insight);
                }
            }
        }

        // 3. Add summary snippet to insights if it's substantial
        if !summary.is_empty() && summary.len() > 20 {
            let clean_summary = crate::engine::orchestrator::utils::format_helpers::strip_grounding_markers(summary);
            let snippet = if clean_summary.len() > 100 { clean_summary[..100].to_string() } else { clean_summary.to_string() };
            if !self.derived_insights.contains(&snippet) {
                self.derived_insights.push(format!("Context: {}", snippet));
            }
        }
    }
}
