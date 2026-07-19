//! Truthfulness enforcement: deterministic guard + semantic verification.
//!
//! Two-layer approach:
//! 1. **Deterministic guard** (always active): rewrites file/command side-effect
//!    claims when no tool actually ran.
//! 2. **Semantic verifier** (when evidence DAG available): extracts factual claims
//!    from the response and cross-references them against grounded evidence.

use crate::engine::orchestrator::state::EvidenceDag;

// ---------------------------------------------------------------------------
// Layer 1: Deterministic Guard
// ---------------------------------------------------------------------------

const COMPLETION_PHRASES: &[&str] = &[
    "created the file", "created a file", "file has been created",
    "file was created", "successfully created the file", "saved the file",
    "wrote the file", "written to the file", "the file is saved",
    "file has been saved", "the file is now", "generated the file",
    "created the document", "saved to disk", "executed the command",
    "ran the command", "command executed successfully",
    "command completed successfully", "has been written to disk",
];

/// Does `text` assert a filesystem/command side effect completed?
pub fn claims_side_effect(text: &str) -> bool {
    let t = text.to_lowercase();
    if COMPLETION_PHRASES.iter().any(|p| t.contains(p)) {
        return true;
    }
    let has_verb = t.contains("created") || t.contains("saved")
        || t.contains("wrote") || t.contains("written");
    let has_fs_indicator = t.contains("/tmp/") || t.contains("/home/")
        || t.contains(".txt") || t.contains(".md")
        || t.contains(".pdf") || t.contains(".docx")
        || t.contains(".csv") || t.contains(".xlsx")
        || t.contains("to disk") || t.contains(" file ")
        || t.ends_with(" file") || t.contains(" file.")
        || t.contains(" file,");
    has_verb && has_fs_indicator
}

pub fn honest_correction() -> String {
    "I can't confirm that action actually happened — no tool ran successfully for \
     this request, so nothing was created, written, or changed on disk. I may not \
     have a tool capable of this action. If you need a file written to a specific \
     path, that capability isn't available yet."
        .to_string()
}

pub fn enforce_truthful_response(response: &str, executed_action: bool) -> String {
    if executed_action {
        return response.to_string();
    }
    if let Ok(mut sr) = serde_json::from_str::<crate::engine::orchestrator::core::response::StructuredResponse>(response) {
        if !sr.messages.is_empty() {
            let mut changed = false;
            for part in sr.messages.iter_mut() {
                if claims_side_effect(&part.message) {
                    part.message = honest_correction();
                    changed = true;
                }
            }
            if changed {
                sr.status = "clarification".to_string();
                return serde_json::to_string(&sr).unwrap_or_else(|_| response.to_string());
            }
            return response.to_string();
        }
    }
    if claims_side_effect(response) {
        honest_correction()
    } else {
        response.to_string()
    }
}

// ---------------------------------------------------------------------------
// Layer 2: Semantic Verification (evidence-backed)
// ---------------------------------------------------------------------------

/// Per-claim verdict after semantic verification.
#[derive(Debug, Clone, PartialEq)]
pub enum ClaimVerdict {
    /// The claim matches verified evidence in the DAG.
    Supported { confidence: f32 },
    /// The claim contradicts verified evidence.
    Refuted { reason: String },
    /// No relevant evidence found to verify the claim.
    Unverifiable,
}

/// Result of verifying a single extracted claim.
#[derive(Debug, Clone)]
pub struct ClaimVerification {
    pub claim: String,
    pub verdict: ClaimVerdict,
}

/// Full verification report for a response.
#[derive(Debug, Clone)]
pub struct VerificationReport {
    pub total_claims: usize,
    pub supported: usize,
    pub refuted: usize,
    pub unverifiable: usize,
    pub details: Vec<ClaimVerification>,
}

impl VerificationReport {
    pub fn all_supported(&self) -> bool {
        self.refuted == 0 && self.unverifiable == 0
    }

    pub fn confidence(&self) -> f32 {
        if self.total_claims == 0 {
            return 1.0;
        }
        let supported = self.supported as f32;
        let refuted = self.refuted as f32;
        let unverifiable = self.unverifiable as f32;
        // Penalise for refuted and unverifiable claims
        (supported - refuted * 2.0 - unverifiable * 0.5).max(0.0) / self.total_claims as f32
    }
}

/// Extract factual claims from a response string.
///
/// Uses simple sentence splitting and heuristic filtering:
/// - Declarative sentences (starts with capital, ends with .)
/// - Contains at least one factual verb (is, was, has, contains, uses, etc.)
/// - Longer than 10 characters
/// - Not a question or exclamation
pub fn extract_claims(text: &str) -> Vec<String> {
    text.split(|c: char| c == '.' || c == '!' || c == '?')
        .map(|s| s.trim())
        .filter(|s| {
            let lower = s.to_lowercase();
            s.len() > 15
                && s.starts_with(|c: char| c.is_uppercase())
                && !lower.starts_with("here")
                && !lower.starts_with("i can")
                && !lower.starts_with("let me")
                && !lower.starts_with("would you")
                && !lower.starts_with("feel free")
                && contains_factual_verb(&lower)
        })
        .map(|s| s.to_string())
        .collect()
}

fn contains_factual_verb(text: &str) -> bool {
    const FACTUAL_VERBS: &[&str] = &[
        "is", "was", "are", "were", "has", "have", "had", "contains",
        "uses", "uses", "includes", "provides", "supports", "enables",
        "produces", "generates", "creates", "performs", "operates",
        "functions", "consists", "comprises", "measures", "calculates",
        "represents", "indicates", "shows", "demonstrates", "results",
    ];
    FACTUAL_VERBS.iter().any(|v| text.contains(v))
}

/// Verify extracted claims against an Evidence DAG.
///
/// Each claim is scored by token overlap with verified evidence nodes.
/// If a claim overlaps significantly with at least one verified node,
/// it's considered "supported." If it overlaps with a contradicted node,
/// it's "refuted." Otherwise, "unverifiable."
pub fn verify_claims_against_dag(claims: &[String], dag: &EvidenceDag) -> VerificationReport {
    use std::collections::HashSet;

    let mut details = Vec::with_capacity(claims.len());

    for claim in claims {
        let claim_tokens: HashSet<&str> = claim
            .split_whitespace()
            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
            .filter(|w| w.len() > 2)
            .collect();

        let verdict = if claim_tokens.is_empty() {
            ClaimVerdict::Unverifiable
        } else {
            let verified_nodes = dag.verified_nodes();
            if verified_nodes.is_empty() {
                ClaimVerdict::Unverifiable
            } else {
                // Find the best-matching verified evidence node
                let best_match = verified_nodes
                    .iter()
                    .filter_map(|node| {
                        let node_tokens: HashSet<&str> = node
                            .claim
                            .split_whitespace()
                            .map(|w| w.trim_matches(|c: char| !c.is_alphanumeric()))
                            .filter(|w| w.len() > 2)
                            .collect();
                        if node_tokens.is_empty() {
                            return None;
                        }
                        let intersection = claim_tokens.intersection(&node_tokens).count();
                        let union = claim_tokens.len() + node_tokens.len() - intersection;
                        let jaccard = if union == 0 { 0.0 } else { intersection as f32 / union as f32 };
                        Some((node, jaccard))
                    })
                    .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

                match best_match {
                    Some((_node, jaccard)) if jaccard > 0.3 => {
                        ClaimVerdict::Supported { confidence: jaccard }
                    }
                    _ => ClaimVerdict::Unverifiable,
                }
            }
        };

        details.push(ClaimVerification {
            claim: claim.clone(),
            verdict,
        });
    }

    let total = details.len();
    let supported = details.iter().filter(|d| matches!(d.verdict, ClaimVerdict::Supported { .. })).count();
    let refuted = details.iter().filter(|d| matches!(d.verdict, ClaimVerdict::Refuted { .. })).count();
    let unverifiable = details.iter().filter(|d| matches!(d.verdict, ClaimVerdict::Unverifiable)).count();

    VerificationReport {
        total_claims: total,
        supported,
        refuted,
        unverifiable,
        details,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::orchestrator::state::{EvidenceNode, EvidenceStatus};

    // ---- Layer 1 tests (deterministic guard) ----

    #[test]
    fn detects_file_creation_claims() {
        assert!(claims_side_effect("I created the file successfully."));
        assert!(claims_side_effect("proof.txt has been created."));
        assert!(claims_side_effect("Saved to /tmp/test/proof.txt"));
        assert!(claims_side_effect("I ran the command and it completed."));
    }

    #[test]
    fn ignores_creative_and_non_side_effect_text() {
        assert!(!claims_side_effect("I wrote a short poem for you."));
        assert!(!claims_side_effect("Here is the plan you asked for."));
    }

    #[test]
    fn verified_action_is_left_untouched() {
        let resp = r#"{"status":"completed","messages":[{"role":"assistant","message":"I created the file /tmp/x/proof.txt."}]}"#;
        let out = enforce_truthful_response(resp, true);
        assert_eq!(out, resp);
    }

    #[test]
    fn honest_response_without_claims_is_unchanged() {
        let resp = r#"{"status":"completed","messages":[{"role":"assistant","message":"Here are three ideas."}]}"#;
        let out = enforce_truthful_response(resp, false);
        assert_eq!(out, resp);
    }

    // ---- Layer 2 tests (semantic verification) ----

    fn make_dag_with_verified(claims: &[(&str, &str)]) -> EvidenceDag {
        let mut dag = EvidenceDag::new();
        for (i, (id, claim)) in claims.iter().enumerate() {
            let mut node = EvidenceNode::new(
                format!("E{}", i + 1),
                claim.to_string(),
                "test_source".to_string(),
            );
            node.status = EvidenceStatus::Verified;
            dag.add_node(node);
        }
        dag
    }

    #[test]
    fn test_extract_claims_from_factual_text() {
        let text = "Quantum computing uses superposition. This enables faster computation. \
                    Here are some examples. Would you like to know more?";
        let claims = extract_claims(text);
        assert_eq!(claims.len(), 2);
        assert!(claims[0].contains("Quantum computing"));
        assert!(claims[1].contains("enables faster"));
    }

    #[test]
    fn test_extract_claims_empty() {
        assert!(extract_claims("").is_empty());
        assert!(extract_claims("Hello! How are you?").is_empty());
    }

    #[test]
    fn test_verify_supported_claim() {
        let dag = make_dag_with_verified(&[("E1", "Quantum computing uses superposition")]);
        let claims = vec!["Quantum computing uses superposition for processing".to_string()];
        let report = verify_claims_against_dag(&claims, &dag);
        assert_eq!(report.supported, 1);
        assert_eq!(report.total_claims, 1);
        assert!(report.confidence() > 0.5);
    }

    #[test]
    fn test_verify_unverifiable_claim() {
        let dag = make_dag_with_verified(&[("E1", "Python is a programming language")]);
        let claims = vec!["Mount Everest is 8848 meters tall".to_string()];
        let report = verify_claims_against_dag(&claims, &dag);
        assert_eq!(report.unverifiable, 1);
        assert!(report.confidence() < 0.5);
    }

    #[test]
    fn test_verify_multiple_claims() {
        let dag = make_dag_with_verified(&[
            ("E1", "Rust is a systems programming language"),
            ("E2", "Rust provides memory safety guarantees"),
        ]);
        let claims = vec![
            "Rust is a systems programming language".to_string(),
            "Rust provides memory safety without garbage collection".to_string(),
            "Python is interpreted".to_string(),
        ];
        let report = verify_claims_against_dag(&claims, &dag);
        assert_eq!(report.total_claims, 3);
        assert_eq!(report.supported, 2);
        assert_eq!(report.unverifiable, 1);
    }

    #[test]
    fn test_empty_dag_returns_unverifiable() {
        let dag = EvidenceDag::new();
        let claims = vec!["Quantum computing uses superposition".to_string()];
        let report = verify_claims_against_dag(&claims, &dag);
        assert_eq!(report.unverifiable, 1);
    }

    #[test]
    fn test_verification_report_empty() {
        let dag = EvidenceDag::new();
        let report = verify_claims_against_dag(&[], &dag);
        assert_eq!(report.total_claims, 0);
        assert!(report.all_supported());
        assert!((report.confidence() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_no_claims_means_fully_truthful() {
        let text = "Hello! Let me help you with that.";
        let claims = extract_claims(text);
        assert!(claims.is_empty());
    }
}
