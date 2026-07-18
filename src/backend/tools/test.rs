#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub async fn test_tool_code(
    language: &str,
    code: &str,
    query: Option<&str>,
) -> anyhow::Result<String> {
    use std::io::Write;
    use tempfile::NamedTempFile;
    use tokio::process::Command;

    let normalized = language.trim().to_lowercase();
    if normalized == "python" {
        let mut temp_file = NamedTempFile::new()?;
        write!(temp_file, "{}", code)?;
        let path = temp_file.path().to_str().unwrap();

        let parts =
            crate::engine::utils::discovery::build_python_command().map_err(anyhow::Error::msg)?;
        let mut cmd = Command::new(&parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }
        cmd.arg(path);
        cmd.kill_on_drop(true);
        #[cfg(windows)]
        {
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        if let Some(q) = query {
            let args: Vec<&str> = q.split_whitespace().collect();
            cmd.args(args);
        }

        let timeout_ms = std::env::var("OVERSEER_TOOL_TEST_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(15_000);

        let output =
            tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), cmd.output())
                .await
                .map_err(|_| anyhow::anyhow!("Tool test timed out after {} ms", timeout_ms))??;

        let mut result = String::new();
        if !output.stdout.is_empty() {
            result.push_str(&String::from_utf8_lossy(&output.stdout));
        }
        if !output.stderr.is_empty() {
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str("--- Error Output ---\n");
            result.push_str(&String::from_utf8_lossy(&output.stderr));
        }

        if result.is_empty() {
            result.push_str("Executed with no output.");
        }
        return Ok(result);
    }

    Ok(format!(
        "Native tool logic for language '{}' is registered. Use the built-in runtime path to exercise real native tools.",
        language
    ))
}
