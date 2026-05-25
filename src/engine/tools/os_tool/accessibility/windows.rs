use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::accessibility::windows_script1::SCRIPT_PART1;
use crate::engine::tools::os_tool::accessibility::windows_script2::SCRIPT_PART2;
use crate::engine::tools::os_tool::powershell::run_powershell_hidden;
use crate::engine::tools::os_tool::uia_selector::build_uia_selector_object;
use crate::engine::tools::os_tool::fields::escape_powershell_single_quoted;

pub async fn windows_accessibility_state_json(
    tool_name: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    let selector_json = build_uia_selector_object(input).to_string();
    let escaped_selector_json = escape_powershell_single_quoted(&selector_json);
    let full_script = format!(
        "{}{}",
        SCRIPT_PART1,
        SCRIPT_PART2
    );
    let script = full_script.replace("'{selector_json}'", &format!("'{}'", escaped_selector_json));

    run_powershell_hidden(&script, tool_name).await
}
