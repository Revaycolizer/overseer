use crate::engine::orchestrator::v3::state::CellState;

pub struct SwarmVerifier {
    pub confidence_threshold: f32,
}

impl SwarmVerifier {
    pub fn new(threshold: f32) -> Self {
        Self { confidence_threshold: threshold }
    }

    pub fn verify_claims(&self, summary: &str, state: &CellState) -> f32 {
        // V3.2: Probabilistic Grounding check
        let sentences: Vec<_> = summary.split('.').collect();
        let mut grounded = 0;
        
        for s in &sentences {
            if self.is_grounded_in_cell(s, state) {
                grounded += 1;
            }
        }
        
        if sentences.is_empty() { 1.0 } else { grounded as f32 / sentences.len() as f32 }
    }

    fn is_grounded_in_cell(&self, sentence: &str, state: &CellState) -> bool {
        // Logic to check if sentence keywords exist in Cell facts or sources
        let lower_s = sentence.to_lowercase();
        state.facts.iter().any(|f| {
            let lower_f = f.to_lowercase();
            lower_s.contains(&lower_f) || lower_f.contains(&lower_s)
        })
    }
}
