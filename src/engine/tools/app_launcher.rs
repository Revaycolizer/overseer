use tokio::process::Command;
use crate::engine::tools::error::DynamicToolError;
use crate::engine::tools::input::object_tool_input;

pub async fn native_app_launcher(args: &serde_json::Value) -> Result<String, DynamicToolError> {
    let input = object_tool_input(args);
    let app = input
        .get("app")
        .and_then(|value| value.as_str())
        .or_else(|| input.get("input").and_then(|value| value.as_str()))
        .ok_or_else(|| DynamicToolError::ExecutionFailed {
            tool: "app_launcher".to_string(),
            details: "Missing app name to launch.".to_string(),
        })?;

    let mut command = if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.arg("/C").arg("start").arg("").arg(app);
        cmd
    } else if cfg!(target_os = "macos") {
        let mut cmd = Command::new("open");
        cmd.arg("-a").arg(app);
        cmd
    } else {
        Command::new(app)
    };

    command.kill_on_drop(false);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    command
        .spawn()
        .map_err(|err| DynamicToolError::ExecutionFailed {
            tool: "app_launcher".to_string(),
            details: err.to_string(),
        })?;

    Ok(format!("Launched '{}'.", app))
}
