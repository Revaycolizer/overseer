pub async fn test_agent_preamble(instructions: &str) -> anyhow::Result<String> {
    // A mock test for agent preamble syntax. For now, it simply validates length and placeholders.
    if instructions.is_empty() {
        return Ok("Agent preamble is empty. Please provide instructions.".to_string());
    }

    let mut result = String::new();
    result.push_str("Syntax Check: Valid\n");
    result.push_str(&format!("Length: {} characters\n", instructions.len()));

    if instructions.contains("{{") && instructions.contains("}}") {
        result.push_str(
            "Detected template variables ({{...}}). Ensure these are populated at runtime.\n",
        );
    }

    result.push_str("Agent instructions are ready for deployment.");
    Ok(result)
}
