use crate::db::Database;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::core::state::StructuredState;
use crate::engine::orchestrator::core::state_update::preview_state_update;
use regex::Regex;

pub async fn compile_response(
    db: &Database,
    step_description: &str,
    result: &SupervisorExecutionResult,
    state: &StructuredState,
) -> Option<SupervisorExecutionResult> {
    println!("🛡️ COMPILING RESPONSE for grounding enforcement (Regex Optimized)...");
    let preview = preview_state_update(state, db, &result.findings, &result.summary, &result.evidence).await;
    
    let sentence_regex = Regex::new(r"(?m)([^.!?]+(?:[.!?](?:\s|$)|$))").unwrap();
    let citation_regex = Regex::new(r"\[E(\d+)\]").unwrap();
    let mut grounded = Vec::new();
    let (mut total, mut matching) = (0, 0);

    let mut found_any = false;
    for caps in sentence_regex.captures_iter(&result.summary) {
        found_any = true;
        let sentence = caps[0].trim().to_string();
        let citations: Vec<String> = citation_regex.captures_iter(&sentence).map(|c| format!("E{}", &c[1])).collect();

        if !citations.is_empty() {
            total += 1;
            if citations.iter().all(|id| preview.evidence.iter().any(|e| &e.id == id)) {
                grounded.push(sentence); matching += 1;
            } else {
                grounded.push(sentence);
            }
        } else {
            grounded.push(sentence);
        }
    }

    if !found_any && !result.summary.is_empty() {
        return None; // Skip compilation, keep original result
    }

    if total > 0 && (matching as f32 / total as f32) < 0.3 {
        return Some(SupervisorExecutionResult::stalled("ABSTAIN | REASON: LOW_CONFIDENCE_GROUNDING".to_string(), "Grounding confidence below 30%".to_string()));
    }

    let mut final_res = result.clone();
    final_res.summary = if grounded.is_empty() { result.summary.clone() } else { grounded.join(" ") };

    final_res.verified = true;
    Some(final_res)
}
