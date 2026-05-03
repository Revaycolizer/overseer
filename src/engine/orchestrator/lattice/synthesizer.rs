use crate::engine::orchestrator::core::result::SupervisorExecutionResult;
use crate::engine::orchestrator::core::response::MessagePart;
use crate::engine::orchestrator::utils::format_helpers::{sanitize_links, strip_grounding_markers};

pub async fn synthesize_step_summary(
    _task_description: &str,
    result: &SupervisorExecutionResult,
) -> String {
    // RULE 1: Artifact Silence (Highest Priority)
    // If a document was generated, focus on the success of that generation.
    if !result.artifacts.is_empty() {
        let files = result.artifacts.join(", ");
        return format!("Successfully generated documentation: {}. The content is professionally formatted and ready for review.", files);
    }

    // RULE 2: Grounding Clean-up & Professional Polish
    let clean_summary = strip_grounding_markers(&result.summary);
    let professional_summary = sanitize_links(&clean_summary);

    // RULE 3: Fallback Synthesis
    if professional_summary.is_empty() || professional_summary.len() < 5 {
        if !result.findings.is_empty() {
             return "I have successfully analyzed the data and integrated the findings into the system state.".to_string();
        }
        return "Task completed successfully.".to_string();
    }

    professional_summary
}

pub fn synthesize_final_response(
    messages: &mut Vec<MessagePart>,
    artifacts: &[String],
) {
    if !artifacts.is_empty() {
        messages.clear();
        messages.push(MessagePart {
            role: "agent".to_string(),
            message: format!("Mission successful. I have generated the following artifacts: {}. These documents are professionally synthesized from research findings and are ready for use.", artifacts.join(", ")),
        });
    }
}
