use crate::engine::orchestrator::core::state::StructuredState;

pub fn build_dispatch_prompt(
    query: &str,
    step: &str,
    tool_list: &str,
    state_str: &str,
    style: &str,
    history: &str,
    retry_msg: Option<&str>
) -> String {
    let retry_inject = match retry_msg {
        Some(msg) => format!("\n\n⚠️ RETRY NOTICE: Your previous response was rejected. REASON: {}\nIMMEDIATE ACTION: You must call a tool NOW. Do NOT plan or describe future intent.", msg),
        None => "".to_string(),
    };

    format!(
        "USER_ORIGINAL_QUERY: {}\nCURRENT_TASK: {}\n\nTOOLS:\n{}\n\nRELEVANT_CONTEXT:\n{}\n\nREPORTING_STYLE: {}\n\nEXECUTION_HISTORY:\n{}\n\n\
         ═══ TRUTH CONTRACT (V3.3) ═══\n\
         1. SUMMARY RULE: Every factual claim in your `summary` MUST conclude with a citation ID [E#].\n\
         2. TOOL INPUT RULE: You are FORBIDDEN from using `[E#]` markers inside tool inputs. Expand them into full text before calling.\n\
         3. DROPPING POLICY: If you lack evidence, discard any unsupported intent.\n\
         4. PHYSICAL REALITY: You MUST call `document_writer` if the task involves file generation.{} \n\n\
         SCHEMA FOR TOOL CALLING:\n{{\"observation\":\"...\",\"intent\":\"...\",\"tool\":\"...\",\"input\":{{...}}}}\n\n\
         Respond with a single JSON block.",
        query, step, tool_list, state_str, style, history, retry_inject
    )
}
