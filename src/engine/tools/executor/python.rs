use crate::engine::tools::error::DynamicToolError;
use crate::engine::utils::discovery::{build_python_command, get_tool_executable_path, ToolInfo};
use tokio::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub async fn execute_python_tool(
    info: &ToolInfo,
    args: serde_json::Value,
) -> Result<String, DynamicToolError> {
    let executable_path =
        get_tool_executable_path(info).ok_or_else(|| DynamicToolError::MissingExecutable {
            tool: info.name.clone(),
        })?;

    let parts =
        build_python_command().map_err(|details| DynamicToolError::MissingPython { details })?;
    let mut cmd = Command::new(&parts[0]);
    if parts.len() > 1 {
        cmd.args(&parts[1..]);
    }
    cmd.arg(&executable_path);
    cmd.kill_on_drop(true);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    let input_str = if let Some(input) = args.get("input") {
        if input.is_string() {
            input.as_str().unwrap_or("").to_string()
        } else {
            input.to_string()
        }
    } else if let Some(inner) = args.get("args") {
        inner.to_string()
    } else if args.is_object() {
        let map = args.as_object().unwrap();
        if map.len() == 1 {
            let val = map.values().next().unwrap();
            if val.is_string() {
                val.as_str().unwrap().to_string()
            } else {
                val.to_string()
            }
        } else if let Some(url) = map.get("url") {
            if url.is_string() {
                url.as_str().unwrap().to_string()
            } else {
                url.to_string()
            }
        } else if let Some(query) = map.get("query") {
            if query.is_string() {
                query.as_str().unwrap().to_string()
            } else {
                query.to_string()
            }
        } else {
            args.to_string()
        }
    } else {
        args.to_string()
    };

    cmd.arg(input_str);

    let output = cmd
        .output()
        .await
        .map_err(|e| DynamicToolError::ExecutionFailed {
            tool: info.name.clone(),
            details: e.to_string(),
        })?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if output.status.success() {
        Ok(stdout)
    } else {
        Err(DynamicToolError::ExecutionFailed {
            tool: info.name.clone(),
            details: format!("{}\n{}", stdout, stderr).trim().to_string(),
        })
    }
}
