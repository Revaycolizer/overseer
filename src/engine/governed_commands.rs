//! Deterministic, model-free governed commands.
//!
//! Slash commands like `/write`, `/open`, `/screenshot` are parsed directly into
//! a typed action and run through `kernel_execute` — the SAME governed path an
//! agent uses (policy → approval → execute → trace). This makes the core "an
//! action is proposed, a human approves, it runs, here's the audit trail" flow
//! work 100% of the time, with no dependency on a local model emitting tool JSON.

use crate::db::Database;
use crate::engine::utils::discovery::ToolInfo;
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq)]
pub enum GovernedCommand {
    /// `/write <path> <content…>`
    Write { path: String, content: String },
    /// `/open <app-or-url>`
    Open { target: String },
    /// `/screenshot`
    Screenshot,
    /// `/do <goal>` (or `/agent <goal>`) — run the native function-calling agent
    /// loop, which plans and executes multiple governed tool calls to reach a goal.
    Agent { goal: String },
}

/// Parse a governed slash-command. Returns `None` for anything else (which then
/// flows to the normal orchestrator).
pub fn parse_governed(input: &str) -> Option<GovernedCommand> {
    let t = input.trim();
    if let Some(rest) = t.strip_prefix("/write ") {
        let rest = rest.trim();
        let (path, content) = match rest.split_once(char::is_whitespace) {
            Some((p, c)) => (p.trim().to_string(), c.trim().to_string()),
            None => (rest.to_string(), String::new()),
        };
        if path.is_empty() {
            return None;
        }
        return Some(GovernedCommand::Write { path, content });
    }
    if let Some(rest) = t.strip_prefix("/open ") {
        let target = rest.trim().to_string();
        if target.is_empty() {
            return None;
        }
        return Some(GovernedCommand::Open { target });
    }
    if t == "/screenshot" {
        return Some(GovernedCommand::Screenshot);
    }
    for prefix in ["/do ", "/agent "] {
        if let Some(rest) = t.strip_prefix(prefix) {
            let goal = rest.trim().to_string();
            if goal.is_empty() {
                return None;
            }
            return Some(GovernedCommand::Agent { goal });
        }
    }
    None
}

fn tool_stub(name: &str) -> ToolInfo {
    ToolInfo {
        id: 0,
        name: name.to_string(),
        description: String::new(),
        code: String::new(),
        language: "rust".to_string(),
        script_path: None,
        required_intent: None,
        safety_level: "standard".to_string(),
        version: "1.0.0".to_string(),
    }
}

/// Run a governed command through the kernel. Creates an execution run, routes
/// the typed action through `kernel_execute` (which enforces policy and pauses
/// for human approval when required), finalizes the run, and returns a
/// `StructuredResponse` JSON string for the chat to render.
pub async fn run_governed(db: &Database, conversation_id: i64, cmd: GovernedCommand) -> String {
    // The agent command isn't a single tool call — hand it to the plan→act→observe
    // loop, which issues its own governed tool calls through the same kernel.
    if let GovernedCommand::Agent { goal } = &cmd {
        return crate::engine::agent_loop::run_agent(db, conversation_id, goal, None).await;
    }

    let (tool_name, args, step): (&str, Value, String) = match &cmd {
        GovernedCommand::Write { path, content } => (
            "write_file",
            json!({ "path": path, "content": content }),
            format!("write {path}"),
        ),
        GovernedCommand::Open { target } => (
            "os_tool",
            json!({ "action": "launch", "target": target }),
            format!("open {target}"),
        ),
        GovernedCommand::Screenshot => (
            "os_tool",
            json!({ "action": "screenshot" }),
            "screenshot".to_string(),
        ),
        GovernedCommand::Agent { .. } => unreachable!("agent command handled above"),
    };

    let run_id = crate::backend::start_execution_run(db, conversation_id, &format!("/{step}"))
        .await
        .ok();

    let tool = tool_stub(tool_name);
    let outcome = crate::engine::kernel::kernel_execute(
        db,
        run_id,
        "computer",
        &step,
        &tool,
        args,
        &[tool_name.to_string()],
    )
    .await;

    let (status, text) = match &outcome {
        overseer_kernel::ExecutionOutcome::Executed { output } => {
            ("completed", format!("✅ Done — {step}.\n\n{}", summarize(output)))
        }
        other => ("failed", other.agent_response()),
    };

    if let Some(rid) = run_id {
        let _ = crate::backend::finish_execution_run(db, rid, status, None, None).await;
    }

    let mut sr = crate::engine::orchestrator::core::response::StructuredResponse::new();
    sr.status = status.to_string();
    sr.messages
        .push(crate::engine::orchestrator::core::response::MessagePart {
            role: "assistant".to_string(),
            message: text,
        });
    serde_json::to_string(&sr).unwrap_or_else(|_| "Done.".to_string())
}

fn summarize(output: &str) -> String {
    if let Ok(v) = serde_json::from_str::<Value>(output) {
        if let Some(s) = v.get("summary").and_then(|s| s.as_str()) {
            return s.to_string();
        }
    }
    output.chars().take(300).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_write_with_path_and_content() {
        let cmd = parse_governed("/write /tmp/x/proof.txt hello there");
        assert_eq!(
            cmd,
            Some(GovernedCommand::Write {
                path: "/tmp/x/proof.txt".into(),
                content: "hello there".into()
            })
        );
    }

    #[test]
    fn parses_open_and_screenshot() {
        assert_eq!(parse_governed("/open firefox"), Some(GovernedCommand::Open { target: "firefox".into() }));
        assert_eq!(parse_governed("/screenshot"), Some(GovernedCommand::Screenshot));
    }

    #[test]
    fn ignores_non_governed_and_incomplete() {
        assert_eq!(parse_governed("hello world"), None);
        assert_eq!(parse_governed("/write"), None); // no path
        assert_eq!(parse_governed("/write   "), None);
        assert_eq!(parse_governed("/open "), None);
    }

    #[tokio::test]
    async fn write_command_creates_run_and_requires_approval() {
        // With approval_required (the default), the write must NOT auto-execute;
        // it creates a run and a pending approval, proving the governed path.
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "t").await.unwrap();
        let dir = std::env::temp_dir().join(format!("overseer-gc-{}", std::process::id()));
        let path = dir.join("proof.txt");
        let _ = std::fs::remove_dir_all(&dir);

        let cmd = GovernedCommand::Write {
            path: path.to_string_lossy().to_string(),
            content: "KERNEL_OK".to_string(),
        };
        let db2 = db.clone();
        let handle = tokio::spawn(async move { run_governed(&db2, conv, cmd).await });

        // A pending kernel approval should appear, and nothing is written yet.
        let mut approved_id = String::new();
        for _ in 0..200 {
            let pending = crate::engine::kernel::list_pending_approvals(&db, None).await;
            if let Some(a) = pending.first() {
                approved_id = a.id.clone();
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(!approved_id.is_empty(), "governed write must create a pending approval");
        assert!(!path.exists(), "file must not exist before approval");

        // Approve -> the staged action executes and writes the file.
        assert!(crate::engine::kernel::approve(&db, &approved_id, "test"));
        let _ = handle.await.unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "KERNEL_OK");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
