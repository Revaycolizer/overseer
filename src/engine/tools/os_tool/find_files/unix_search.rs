use std::path::PathBuf;
use tokio::process::Command;
use crate::engine::tools::error::DynamicToolError;

pub fn get_unix_search_roots() -> Vec<PathBuf> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut roots = vec![std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))];
    for candidate in [
        format!("{}/Desktop", home),
        format!("{}/Documents", home),
        format!("{}/Downloads", home),
        format!("{}/OneDrive", home),
    ] {
        let path = PathBuf::from(candidate);
        if path.exists() {
            roots.push(path);
        }
    }
    roots.sort();
    roots.dedup();
    roots
}

pub async fn execute_unix_search(
    roots: &[PathBuf],
    tool_name: &str,
) -> Result<Vec<String>, DynamicToolError> {
    let mut file_lines = Vec::new();
    if Command::new("rg").arg("--version").output().await.is_ok() {
        let mut cmd = Command::new("rg");
        cmd.arg("--files");
        for root in roots {
            cmd.arg(root);
        }

        if let Ok(output) = cmd.output().await {
            file_lines.extend(
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    .map(|line| line.to_string()),
            );
        }
    }

    if file_lines.is_empty() {
        let mut cmd = Command::new("find");
        for root in roots {
            cmd.arg(root);
        }
        cmd.args(["-type", "f"]);

        let output = cmd
            .output()
            .await
            .map_err(|e| DynamicToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                details: e.to_string(),
            })?;

        file_lines.extend(
            String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(|line| line.to_string()),
        );
    }

    Ok(file_lines)
}
