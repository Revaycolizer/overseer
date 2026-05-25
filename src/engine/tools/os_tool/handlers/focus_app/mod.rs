pub mod focus;
pub mod desktop_file;
pub mod launch;
pub mod url_wait;

use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::os_tool::target_extraction::extract_os_tool_target;
use crate::engine::tools::os_tool::uia_interaction::windows_uia_interaction_json;
use crate::engine::tools::input::read_i64_field;
use crate::engine::tools::os_tool::handlers::focus_app::focus::handle_focus;
use crate::engine::tools::os_tool::handlers::focus_app::launch::handle_launch;
use crate::engine::tools::os_tool::handlers::focus_app::url_wait::{handle_open_url, handle_wait};

pub async fn handle_focus_app_action(
    action: &str,
    input: &serde_json::Value,
    selector_present: bool,
) -> Result<String, DynamicToolError> {
    match action {
        "focus" => {
            if selector_present {
                windows_uia_interaction_json("os_tool", "focus_element", input).await
            } else {
                let target = extract_os_tool_target(input, "focus");
                handle_focus(&target).await
            }
        }
        "focus_element" => {
            windows_uia_interaction_json("os_tool", "focus_element", input).await
        }
        "invoke_element" => {
            windows_uia_interaction_json("os_tool", "invoke_element", input).await
        }
        "launch" => {
            let app = extract_os_tool_target(input, "launch");
            handle_launch(&app).await
        }
        "open_url" => {
            let url = extract_os_tool_target(input, "open_url");
            if !(url.starts_with("http://") || url.starts_with("https://")) {
                return Err(DynamicToolError::ExecutionFailed {
                    tool: "os_tool".to_string(),
                    details: "Open URL only supports full http:// or https:// links.".to_string(),
                });
            }
            handle_open_url(&url).await
        }
        "wait" => {
            let duration_ms = read_i64_field(input, &["milliseconds", "ms", "duration_ms", "duration"])
                .unwrap_or(1000)
                .clamp(0, 30_000) as u64;
            handle_wait(duration_ms).await
        }
        _ => Err(DynamicToolError::ExecutionFailed {
            tool: "os_tool".to_string(),
            details: format!("Unsupported focus_app action '{}'", action),
        }),
    }
}
