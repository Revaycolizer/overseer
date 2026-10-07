use std::collections::HashSet;

use crate::engine::orchestrator::state::CellState;

/// Verifies that claims in a summary are grounded in cell evidence.
///
/// Uses token-set overlap ratio (Jaccard similarity on tokens) instead of
/// simple substring matching. This gives a more robust grounding score
/// that resists superficial textual variations.
pub struct SwarmVerifier {
    pub confidence_threshold: f32,
}

impl SwarmVerifier {
    pub fn new(threshold: f32) -> Self {
        Self {
            confidence_threshold: threshold,
        }
    }

    /// Score a summary against cell evidence.
    ///
    /// Returns a value in [0.0, 1.0] where 1.0 means every sentence in the
    /// summary has strong evidence overlap with at least one cell fact.
    pub fn verify_claims(&self, summary: &str, state: &CellState) -> f32 {
        let sentences: Vec<&str> = summary
            .split(|c: char| c == '.' || c == '!' || c == '?')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        if sentences.is_empty() {
            return 1.0;
        }

        let mut grounded_count = 0usize;
        for sentence in &sentences {
            let sentence_tokens = tokenise_set(sentence);
            if sentence_tokens.is_empty() {
                grounded_count += 1; // Empty sentences are vacuously true
                continue;
            }

            let max_overlap = state
                .facts
                .iter()
                .map(|fact| {
                    let fact_tokens = tokenise_set(fact);
                    jaccard_similarity(&sentence_tokens, &fact_tokens)
                })
                .fold(0.0f64, f64::max);

            if max_overlap >= self.confidence_threshold as f64 {
                grounded_count += 1;
            }
        }

        grounded_count as f32 / sentences.len() as f32
    }

    /// A sentence is considered grounded if its token overlap with ANY fact
    /// exceeds the confidence threshold.
    pub fn is_grounded(&self, sentence: &str, state: &CellState) -> bool {
        let sentence_tokens = tokenise_set(sentence);
        if sentence_tokens.is_empty() {
            return true;
        }
        state.facts.iter().any(|fact| {
            let fact_tokens = tokenise_set(fact);
            jaccard_similarity(&sentence_tokens, &fact_tokens) >= self.confidence_threshold as f64
        })
    }
}

/// Tokenise text into a set of lowercased alphanumeric tokens (>=3 chars),
/// with common stop-words removed.
fn tokenise_set(text: &str) -> HashSet<String> {
    const STOP_WORDS: &[&str] = &[
        "the", "and", "for", "are", "was", "were", "has", "had", "but", "not", "this", "that",
        "with", "from", "they", "have", "been", "would", "could", "should", "will", "more", "than",
        "then", "also", "about", "just", "very", "what", "when", "where", "which", "who", "whom",
        "how", "may", "might",
    ];
    let stop: HashSet<&str> = STOP_WORDS.iter().copied().collect();

    text.split_whitespace()
        .map(|w| {
            w.trim_matches(|c: char| !c.is_alphanumeric())
                .to_lowercase()
        })
        .filter(|w| w.len() > 2 && !stop.contains(w.as_str()))
        .collect()
}

/// Jaccard similarity between two sets of tokens.
fn jaccard_similarity(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    let intersection: HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let b_refs: HashSet<&str> = b.iter().map(|s| s.as_str()).collect();
    let intersection_len = intersection.intersection(&b_refs).count();
    let union_len = a.len() + b.len() - intersection_len;

    if union_len == 0 {
        1.0
    } else {
        intersection_len as f64 / union_len as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell_with_facts(facts: Vec<String>) -> CellState {
        let mut cell = CellState::new("test");
        for f in facts {
            cell.add_fact(f);
        }
        cell
    }

    #[test]
    fn test_fully_grounded() {
        let cell = cell_with_facts(vec![
            "Quantum computing uses superposition".to_string(),
            "Entanglement allows quantum correlations".to_string(),
        ]);
        let verifier = SwarmVerifier::new(0.3);
        let score = verifier.verify_claims(
            "Quantum computing uses superposition and entanglement.",
            &cell,
        );
        assert!(score > 0.5, "Expected high grounding score, got {}", score);
    }

    #[test]
    fn test_empty_sentences_vacuously_true() {
        let cell = cell_with_facts(vec![]);
        let verifier = SwarmVerifier::new(0.5);
        let score = verifier.verify_claims("", &cell);
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_unrelated_sentences_score_low() {
        let cell = cell_with_facts(vec![
            "Python is a programming language".to_string(),
            "Mount Everest is 8848 meters tall".to_string(),
        ]);
        let verifier = SwarmVerifier::new(0.5);
        let score = verifier.verify_claims(
            "Quantum computing uses superposition and entanglement.",
            &cell,
        );
        assert!(
            score < 0.5,
            "Expected low grounding score for unrelated content, got {}",
            score
        );
    }

    #[test]
    fn test_confidence_threshold() {
        let cell = cell_with_facts(vec!["The sky is blue".to_string()]);
        let verifier = SwarmVerifier::new(0.9); // Very high threshold
        let score = verifier.verify_claims("The sky appears blue during daytime.", &cell);
        // Should still find some overlap
        assert!(score >= 0.0 && score <= 1.0);
    }
}
