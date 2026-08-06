//! Native function-calling agent loop (the "brain").
//!
//! Takes a natural-language goal and runs a **plan → act → observe** loop against
//! a cloud provider with real function-calling: the model picks a tool, we run it
//! through `kernel_execute` (policy → approval → execute → trace), feed the result
//! back as an observation, and repeat until the model returns a final answer.
//!
//! Every tool call goes through the SAME kernel chokepoint as a governed slash
//! command, so policy and human approval still apply — the model can *propose*
//! actions but cannot bypass the Kernel. This is the reliable replacement for the
//! old "hope the local model emits valid tool JSON in its text" path.

use crate::db::Database;
use crate::engine::providers::{self, AgentMessage, ToolCall, ToolSpec};
use crate::engine::utils::discovery::ToolInfo;
use serde_json::{json, Value};

/// Max plan→act→observe iterations before we stop and summarize.
const MAX_STEPS: usize = 8;
/// Clamp on observation text fed back to the model per tool call.
const MAX_OBSERVATION: usize = 6_000;

const SYSTEM_PROMPT: &str = "\
You are Overseer, an assistant that operates the user's computer to accomplish goals.
You act by calling the provided tools. Work in small, verifiable steps: call a tool,
read its result, then decide the next step. Prefer `shell_exec` for file and system
work (listing, reading, moving, deleting, searching, creating directories). Use
`fs_write` to create or overwrite a file's contents. Use `app_open` to launch an app
or open a URL, and `screenshot` to capture the screen.

Every action is subject to human approval and is audited — do not attempt to bypass
this. When the goal is complete, stop calling tools and reply with a short summary of
what you did. If a tool is rejected or fails, adapt or explain what is blocked.";

/// Build the Tier-1 tool catalog exposed to the model. Each maps to an existing
/// Kernel-governed native tool at dispatch time.
pub fn tool_catalog() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "shell_exec".into(),
            description: "Run a shell command and get stdout/stderr/exit code. Use for listing, reading, moving, copying, deleting, searching files, and general system tasks.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "The shell command to run." },
                    "cwd": { "type": "string", "description": "Optional working directory." }
                },
                "required": ["command"]
            }),
        },
        ToolSpec {
            name: "fs_write".into(),
            description: "Write (create or overwrite) a file with the given text content. Creates parent directories as needed.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Absolute or ~-relative file path." },
                    "content": { "type": "string", "description": "Full file content to write." }
                },
                "required": ["path", "content"]
            }),
        },
        ToolSpec {
            name: "app_open".into(),
            description: "Launch an application by name, or open a URL/file with the default handler.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string", "description": "App name, URL, or file path." }
                },
                "required": ["target"]
            }),
        },
        ToolSpec {
            name: "screenshot".into(),
            description: "Capture a screenshot of the current screen.".into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
    ]
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

/// Map a model tool call to the underlying native tool + kernel domain.
/// Returns `(native_tool_name, arguments, domain, step_label)`.
/// The domain matters: it drives `classify_action_kind` (e.g. "shell" → ShellCommand,
/// "computer" → ComputerControl), which decides which approval policy applies.
fn map_call(call: &ToolCall) -> Option<(&'static str, Value, &'static str, String)> {
    let a = &call.arguments;
    match call.name.as_str() {
        "shell_exec" => {
            let command = a.get("command").and_then(|v| v.as_str()).unwrap_or_default();
            Some(("shell_exec", a.clone(), "shell", format!("shell: {command}")))
        }
        "fs_write" => {
            let path = a.get("path").and_then(|v| v.as_str()).unwrap_or_default();
            Some(("write_file", a.clone(), "filesystem", format!("write {path}")))
        }
        "app_open" => {
            let target = a.get("target").and_then(|v| v.as_str()).unwrap_or_default();
            Some((
                "os_tool",
                json!({ "action": "launch", "target": target }),
                "computer",
                format!("open {target}"),
            ))
        }
        "screenshot" => Some((
            "os_tool",
            json!({ "action": "screenshot" }),
            "computer",
            "screenshot".to_string(),
        )),
        _ => None,
    }
}

/// Execute one tool call through the kernel and return an observation string.
async fn dispatch_tool(db: &Database, run_id: Option<i64>, call: &ToolCall) -> String {
    let Some((tool_name, args, domain, step)) = map_call(call) else {
        return format!("unknown tool '{}'", call.name);
    };
    let tool = tool_stub(tool_name);
    let outcome = crate::engine::kernel::kernel_execute(
        db,
        run_id,
        domain,
        &step,
        &tool,
        args,
        &[tool_name.to_string()],
    )
    .await;

    let mut text = match &outcome {
        overseer_kernel::ExecutionOutcome::Executed { output } => output.clone(),
        other => other.agent_response(),
    };
    if text.len() > MAX_OBSERVATION {
        text.truncate(MAX_OBSERVATION);
        text.push_str("\n…(truncated)");
    }
    text
}

/// Run the agent loop for a goal. Returns a `StructuredResponse` JSON string for
/// the chat to render (same shape as governed commands).
pub async fn run_agent(
    db: &Database,
    conversation_id: i64,
    goal: &str,
    token_sender: Option<tokio::sync::mpsc::UnboundedSender<String>>,
) -> String {
    let cfg = providers::load_config(db).await;
    if !cfg.is_cloud_ready() {
        return respond(
            "failed",
            "Agent mode needs a cloud model. Set a provider and API key in Settings → Model Provider (OpenAI or Anthropic), then try `/do <goal>` again.",
        );
    }

    let run_id = crate::backend::start_execution_run(db, conversation_id, &format!("/do {goal}"))
        .await
        .ok();
    let tools = tool_catalog();
    let mut history = vec![AgentMessage::User(goal.to_string())];
    let mut transcript = String::new();

    for _step in 0..MAX_STEPS {
        let turn = match providers::complete_with_tools(&cfg, SYSTEM_PROMPT, &history, &tools).await {
            Ok(t) => t,
            Err(e) => {
                finish(db, run_id, "failed").await;
                return respond("failed", &format!("Model error: {e}"));
            }
        };

        // No tool calls → the model is done.
        if turn.tool_calls.is_empty() {
            let final_text = turn.text.unwrap_or_else(|| "Done.".to_string());
            finish(db, run_id, "completed").await;
            let body = if transcript.is_empty() {
                final_text
            } else {
                format!("{final_text}\n\n---\n{}", transcript.trim_end())
            };
            return respond("completed", &body);
        }

        history.push(AgentMessage::Assistant {
            text: turn.text.clone(),
            tool_calls: turn.tool_calls.clone(),
        });

        for call in &turn.tool_calls {
            let label = map_call(call)
                .map(|(_, _, _, step)| step)
                .unwrap_or_else(|| call.name.clone());
            if let Some(tx) = token_sender.as_ref() {
                let _ = tx.send(format!("→ {label}\n"));
            }
            let observation = dispatch_tool(db, run_id, call).await;
            let one_line = observation.lines().next().unwrap_or("").trim();
            transcript.push_str(&format!("• {label} — {}\n", short(one_line, 160)));
            history.push(AgentMessage::ToolResult {
                id: call.id.clone(),
                name: call.name.clone(),
                content: observation,
            });
        }
    }

    finish(db, run_id, "completed").await;
    respond(
        "completed",
        &format!(
            "Reached the {MAX_STEPS}-step limit.\n\n---\n{}",
            transcript.trim_end()
        ),
    )
}

async fn finish(db: &Database, run_id: Option<i64>, status: &str) {
    if let Some(rid) = run_id {
        let _ = crate::backend::finish_execution_run(db, rid, status, None, None).await;
    }
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n {
        s.to_string()
    } else {
        let mut t: String = s.chars().take(n).collect();
        t.push('…');
        t
    }
}

fn respond(status: &str, message: &str) -> String {
    let mut sr = crate::engine::orchestrator::core::response::StructuredResponse::new();
    sr.status = status.to_string();
    sr.messages
        .push(crate::engine::orchestrator::core::response::MessagePart {
            role: "assistant".to_string(),
            message: message.to_string(),
        });
    serde_json::to_string(&sr).unwrap_or_else(|_| message.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_has_tier1_tools() {
        let names: Vec<String> = tool_catalog().into_iter().map(|t| t.name).collect();
        for expected in ["shell_exec", "fs_write", "app_open", "screenshot"] {
            assert!(names.contains(&expected.to_string()), "missing {expected}");
        }
    }

    #[test]
    fn shell_calls_route_to_shell_domain_for_shellcommand_kind() {
        // domain must NOT be computer/desktop, or classify_action_kind would tag
        // it ComputerControl instead of ShellCommand.
        let call = ToolCall {
            id: "1".into(),
            name: "shell_exec".into(),
            arguments: json!({ "command": "ls" }),
        };
        let (tool, _args, domain, _step) = map_call(&call).unwrap();
        assert_eq!(tool, "shell_exec");
        assert_eq!(domain, "shell");
    }

    #[test]
    fn app_open_transforms_to_os_tool_launch() {
        let call = ToolCall {
            id: "1".into(),
            name: "app_open".into(),
            arguments: json!({ "target": "firefox" }),
        };
        let (tool, args, domain, _) = map_call(&call).unwrap();
        assert_eq!(tool, "os_tool");
        assert_eq!(domain, "computer");
        assert_eq!(args["action"], "launch");
        assert_eq!(args["target"], "firefox");
    }

    #[tokio::test]
    async fn without_cloud_key_explains_how_to_enable() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "t").await.unwrap();
        let out = run_agent(&db, conv, "list my home dir", None).await;
        assert!(out.contains("Settings"), "should guide user to configure a provider");
    }
}
