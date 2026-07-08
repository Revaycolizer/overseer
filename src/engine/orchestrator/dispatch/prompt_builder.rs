
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
        "═══ TRUTH CONTRACT (V3.3) ═══\n\
         1. SUMMARY RULE: Every factual claim in your `summary` MUST conclude with a citation ID [E#].\n\
         2. TOOL INPUT RULE: You are FORBIDDEN from using `[E#]` markers inside tool inputs. Expand them into full text before calling.\n\
         3. DROPPING POLICY: If you lack evidence, discard any unsupported intent.\n\
         4. PHYSICAL REALITY: You MUST call `document_writer` if the task involves file generation.\n\n\
         SCHEMA FOR TOOL CALLING:\n{{\"observation\":\"...\",\"intent\":\"...\",\"tool\":\"...\",\"input\":{{...}}}}\n\n\
         Respond with a single JSON block.\n\n\
         REPORTING_STYLE: {}\n\
         TOOLS:\n{}\n\n\
         USER_ORIGINAL_QUERY: {}\n\
         CURRENT_TASK: {}\n\n\
         RELEVANT_CONTEXT:\n{}\n\n\
         EXECUTION_HISTORY:\n{}{}",
        style, tool_list, query, step, state_str, history, retry_inject
    )
}
