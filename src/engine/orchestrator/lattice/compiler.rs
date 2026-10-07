use crate::db::Database;
use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::state::preview_state_update;
use crate::engine::orchestrator::state::StructuredState;
use regex::Regex;

pub async fn compile_response(
    db: &Database,
    _step_description: &str,
    result: &SupervisorExecutionResult,
    state: &StructuredState,
) -> Option<SupervisorExecutionResult> {
    tracing::debug!("Compiling response for grounding enforcement");
    let preview = preview_state_update(
        state,
        db,
        &result.findings,
        &result.summary,
        &result.evidence,
    )
    .await;

    let sentence_regex = Regex::new(r"(?m)([^.!?]+(?:[.!?](?:\s|$)|$))").unwrap();
    let citation_regex = Regex::new(r"\[E(\d+)\]").unwrap();
    let mut grounded = Vec::new();
    let (mut total, mut matching) = (0, 0);

    for caps in sentence_regex.captures_iter(&result.summary) {
        let sentence = caps[0].trim().to_string();
        let citations: Vec<String> = citation_regex
            .captures_iter(&sentence)
            .map(|c| format!("E{}", &c[1]))
            .collect();

        if !citations.is_empty() {
            total += 1;
            if citations
                .iter()
                .all(|id| preview.evidence.iter().any(|e| &e.id == id))
            {
                grounded.push(sentence);
                matching += 1;
            }
        } else if is_conversational(&sentence) {
            grounded.push(sentence);
        }
    }

    if total > 0 && (matching as f32 / total as f32) < 0.5 && result.artifacts.is_empty() {
        return Some(SupervisorExecutionResult::stalled(
            "ABSTAIN | REASON: INSUFFICIENT_EVIDENCE".to_string(),
            "Grounding failed".to_string(),
        ));
    }

    let mut final_res = result.clone();

    // HEURISTIC: If grounded is empty but we have artifacts, we must preserve the artifact summary
    if grounded.is_empty() && !result.artifacts.is_empty() {
        final_res.summary = result.summary.clone();
    } else if !grounded.is_empty() {
        final_res.summary = grounded.join(" ");
    }

    final_res.verified = true;
    Some(final_res)
}

fn is_conversational(s: &str) -> bool {
    !s.chars().any(|c| c.is_ascii_digit()) || s.len() < 40
}
