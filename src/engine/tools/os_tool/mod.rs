pub mod accessibility;
pub mod action_groups;
pub mod action_inference;
pub mod browser_state;
pub mod desktop_state;
pub mod fields;
pub mod find_files;
pub mod handlers;
pub mod input_helpers;
pub mod open_file;
pub mod powershell;
pub mod process_list;
pub mod read_file;
pub mod screenshot;
pub mod target_extraction;
pub mod uia_inference;
pub mod uia_interaction;
pub mod uia_selector;
pub mod unix_command;
pub mod wayland_compat;
pub mod workspace_search;

#[cfg(test)]
pub mod tests;

use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::object_tool_input;
use crate::engine::tools::os_tool::action_inference::infer_os_tool_action;
use crate::engine::tools::os_tool::handlers::files::handle_files_action;
use crate::engine::tools::os_tool::handlers::focus_app::handle_focus_app_action;
use crate::engine::tools::os_tool::handlers::input::hotkey::handle_hotkey;
use crate::engine::tools::os_tool::handlers::input::keyboard::handle_keyboard_action;
use crate::engine::tools::os_tool::handlers::input::mouse::handle_mouse_action;
use crate::engine::tools::os_tool::handlers::input::scroll::handle_scroll;
use crate::engine::tools::os_tool::handlers::state::handle_state_action;
use crate::engine::tools::os_tool::uia_selector::has_uia_selector;

pub async fn native_os_tool(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let action = infer_os_tool_action(&input);
    let selector_present = has_uia_selector(&input);

    match action.as_str() {
        "get_state" | "list_windows" | "inspect_accessibility" | "browser_state" | "list_processes" => {
            handle_state_action(&action, &input).await
        }
        "find_files" | "read_file" | "open_file" | "screenshot" => {
            handle_files_action(&action, &input).await
        }
        "focus" | "focus_element" | "invoke_element" | "launch" | "open_url" | "wait" => {
            handle_focus_app_action(&action, &input, selector_present).await
        }
        "click" | "double_click" | "right_click" | "move_mouse" => {
            handle_mouse_action(&action, &input, selector_present).await
        }
        "type_text" | "press_key" => {
            handle_keyboard_action(&action, &input, selector_present).await
        }
        "scroll" => {
            handle_scroll(&input).await
        }
        "hotkey" => {
            handle_hotkey(&input).await
        }
        other => Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!(
                "Unsupported os_tool action '{}'. Supported actions: get_state, list_windows, inspect_accessibility, browser_state, list_processes, find_files, read_file, open_file, screenshot, focus, focus_element, invoke_element, click, double_click, right_click, move_mouse, type_text, press_key, hotkey, scroll, launch, open_url, wait.",
                other
            ),
        }),
    }
}
