use crate::engine::tools::input::{read_string_field, stringify_tool_input};
use crate::engine::tools::os_tool::action_groups::{infer_action_group_1, infer_action_group_2, infer_action_group_3};

pub fn infer_os_tool_action(input: &serde_json::Value) -> String {
    let explicit = read_string_field(input, &["action", "command", "mode", "query"])
        .or_else(|| read_string_field(input, &["input"]))
        .unwrap_or_else(|| stringify_tool_input(input));
    let lower = explicit.trim().to_lowercase();

    if lower.is_empty() {
        return "get_state".to_string();
    }

    if let Some(action) = infer_action_group_1(&lower) {
        return action;
    }
    if let Some(action) = infer_action_group_2(&lower) {
        return action;
    }
    if let Some(action) = infer_action_group_3(&lower) {
        return action;
    }

    "get_state".to_string()
}
