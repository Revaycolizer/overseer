use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::uia_interaction::windows_script1::SCRIPT_PART1;
use crate::engine::tools::os_tool::uia_interaction::windows_script2::SCRIPT_PART2;
use crate::engine::tools::os_tool::uia_interaction::windows_script3::SCRIPT_PART3;
use crate::engine::tools::os_tool::uia_interaction::windows_script4::SCRIPT_PART4;
use crate::engine::tools::os_tool::powershell::run_powershell_hidden;
use crate::engine::tools::os_tool::uia_selector::build_uia_selector_object;
use crate::engine::tools::os_tool::input_helpers::extract_text_value;
use crate::engine::tools::os_tool::fields::escape_powershell_single_quoted;

pub async fn windows_uia_interaction_json(
    tool_name: &str,
    action: &str,
    input: &serde_json::Value,
) -> Result<String, DynamicToolError> {
    let selector_json = build_uia_selector_object(input).to_string();
    let escaped_selector_json = escape_powershell_single_quoted(&selector_json);
    let escaped_action = escape_powershell_single_quoted(action);
    let interaction_text = if action == "type_text" {
        extract_text_value(input).unwrap_or_default()
    } else {
        String::new()
    };
    let escaped_text = escape_powershell_single_quoted(&interaction_text);

    let full_script = format!(
        "{}{}{}{}",
        SCRIPT_PART1,
        SCRIPT_PART2,
        SCRIPT_PART3,
        SCRIPT_PART4
    );

    let script = full_script
        .replace("'{selector_json}'", &format!("'{}'", escaped_selector_json))
        .replace("'{action_name}'", &format!("'{}'", escaped_action))
        .replace("'{text_value}'", &format!("'{}'", escaped_text));

    run_powershell_hidden(&script, tool_name).await
}
