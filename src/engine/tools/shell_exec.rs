//! Governed shell-execution tool.
//!
//! Runs a single shell command and captures stdout/stderr/exit code. This is the
//! Tier-1 "hands" primitive that covers most file and sysadmin work (ls, cat, mv,
//! rm, mkdir, grep, find, …) without a bespoke tool per verb.
//!
//! It is side-effecting and powerful, so at the dispatch chokepoint it is
//! classified as [`ActionKind::ShellCommand`](overseer_kernel::ActionKind) and
//! governed by the kernel (approval-required by default). This function performs
//! the run only after the kernel has authorized it — callers must go through
//! `kernel_execute`, never call this directly for agent actions.

use crate::engine::tools::error::DynamicToolError;
use serde_json::{json, Value};
use std::time::Duration;

fn fail(details: impl Into<String>) -> DynamicToolError {
    DynamicToolError::ExecutionFailed {
        tool: "shell_exec".to_string(),
        details: details.into(),
    }
}

const DEFAULT_TIMEOUT_MS: u64 = 60_000;
const MAX_CAPTURE: usize = 200_000; // clamp captured stdout/stderr

/// Execute a `shell_exec` action. Accepts `{ "command": "...", "cwd"?: "...",
/// "timeout_ms"?: N }` either at the top level or nested under `"input"`.
pub async fn native_shell_exec(args: &Value) -> Result<String, DynamicToolError> {
    let nested = args.get("input").filter(|v| v.is_object());
    let get = |key: &str| -> Option<Value> {
        nested
            .and_then(|n| n.get(key))
            .or_else(|| args.get(key))
            .cloned()
    };

    let command = get("command")
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| fail("missing required 'command'"))?;

    let cwd = get("cwd").and_then(|v| v.as_str().map(|s| s.to_string()));
    let timeout_ms = get("timeout_ms")
        .and_then(|v| v.as_u64())
        .unwrap_or(DEFAULT_TIMEOUT_MS);

    let mut cmd = build_command(&command);
    if let Some(dir) = cwd.as_ref().filter(|d| !d.trim().is_empty()) {
        cmd.current_dir(dir);
    }
    cmd.stdin(std::process::Stdio::null());

    let run = cmd.output();
    let output = match tokio::time::timeout(Duration::from_millis(timeout_ms), run).await {
        Ok(Ok(o)) => o,
        Ok(Err(e)) => return Err(fail(format!("failed to run command: {e}"))),
        Err(_) => {
            return Err(DynamicToolError::TimedOut {
                tool: "shell_exec".to_string(),
                timeout_ms,
            })
        }
    };

    let mut stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let mut stderr = String::from_utf8_lossy(&output.stderr).to_string();
    stdout.truncate(MAX_CAPTURE);
    stderr.truncate(MAX_CAPTURE);
    let code = output.status.code();
    let ok = output.status.success();

    let summary = if ok {
        let first = stdout.lines().next().unwrap_or("").trim();
        if first.is_empty() {
            format!("Ran `{command}` (exit 0)")
        } else {
            format!("Ran `{command}` → {first}")
        }
    } else {
        let why = stderr.lines().next().unwrap_or("").trim();
        format!("`{command}` exited {} — {}", code.unwrap_or(-1), why)
    };

    Ok(json!({
        "status": if ok { "completed" } else { "failed" },
        "tool": "shell_exec",
        "command": command,
        "exit_code": code,
        "stdout": stdout,
        "stderr": stderr,
        "summary": summary,
    })
    .to_string())
}

#[cfg(target_family = "unix")]
fn build_command(command: &str) -> tokio::process::Command {
    let mut c = tokio::process::Command::new("/bin/sh");
    c.arg("-c").arg(command);
    c
}

#[cfg(target_family = "windows")]
fn build_command(command: &str) -> tokio::process::Command {
    let mut c = tokio::process::Command::new("cmd");
    c.arg("/C").arg(command);
    c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn runs_command_and_captures_stdout() {
        let out = native_shell_exec(&json!({ "command": "echo overseer_ok" }))
            .await
            .expect("echo should run");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["status"], "completed");
        assert_eq!(v["exit_code"], 0);
        assert!(v["stdout"].as_str().unwrap().contains("overseer_ok"));
    }

    #[tokio::test]
    async fn nonzero_exit_is_failed_not_error() {
        let out = native_shell_exec(&json!({ "command": "exit 3" }))
            .await
            .expect("should return Ok with failed status, not Err");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["status"], "failed");
        assert_eq!(v["exit_code"], 3);
    }

    #[tokio::test]
    async fn missing_command_is_an_error() {
        assert!(native_shell_exec(&json!({ "cwd": "/tmp" })).await.is_err());
    }

    #[tokio::test]
    async fn respects_cwd() {
        let out = native_shell_exec(&json!({ "command": "pwd", "cwd": "/tmp" }))
            .await
            .expect("pwd should run");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert!(v["stdout"].as_str().unwrap().contains("/tmp"));
    }
}
