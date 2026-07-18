use crate::engine::tools::input::{read_string_field, stringify_tool_input};
use crate::engine::tools::os_tool::target_extraction::group1::{extract_target_group_1, extract_target_group_2};
use crate::engine::tools::os_tool::target_extraction::group2::extract_target_group_3;

pub fn extract_os_tool_target(input: &serde_json::Value, action: &str) -> String {
    let explicit = read_string_field(input, &["target", "element", "selector", "path", "query", "app", "url", "text", "key"])
        .or_else(|| read_string_field(input, &["input"]))
        .unwrap_or_else(|| stringify_tool_input(input));
    let lower = explicit.trim().to_lowercase();

    if let Some(target) = extract_target_group_1(action, &lower, &explicit) {
        return target;
    }
    if let Some(target) = extract_target_group_2(action, &lower, &explicit) {
        return target;
    }
    if let Some(target) = extract_target_group_3(action, &lower, &explicit) {
        return target;
    }

    explicit.trim().to_string()
}
