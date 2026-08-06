//! Native function-calling agent loop (the "brain").
//!
//! Takes a natural-language goal and runs a **plan → act → observe** loop against
//! the configured model — **local Ollama by default**, cloud (OpenAI/Anthropic)
//! only if selected — using real function-calling: the model picks a tool, we run it
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
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::Mutex;

/// Max plan→act→observe iterations before we stop and summarize.
const MAX_STEPS: usize = 8;
/// Clamp on observation text fed back to the model per tool call.
const MAX_OBSERVATION: usize = 6_000;

const SYSTEM_PROMPT: &str = "\
You are Overseer, an assistant that operates the user's computer to accomplish goals.
You act by calling the provided tools. Work in small, verifiable steps: call a tool,
read its result, then decide the next step.

Choosing tools:
- Files & system: prefer `shell_exec` (list, read, move, copy, delete, search, mkdir).
  Use `fs_write` to create/overwrite a file's contents.
- Apps & windows: `app_open` to launch an app or open a URL/file; `focus_window` to
  bring a window forward; `list_windows` to see what's open.
- The web: `read_webpage` to fetch and read a page's text.
- GUI interaction: `see` FIRST to find on-screen coordinates, then `click`/`move_mouse`/
  `type_text`/`press_key`/`hotkey`/`scroll`. Never guess coordinates blindly — look first.
- Memory: `remember` durable facts/preferences/results; `recall` them later (persists
  across sessions).

Verify as you go: after an action, check the result (or `see`/`shell_exec`) before moving on.
Every action is subject to human approval and is audited — do not attempt to bypass this.
When the goal is complete, stop calling tools and reply with a short summary of what you
did. If a tool is rejected or fails, adapt or explain what is blocked.";

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
        // ---- Perception (see) ----
        ToolSpec {
            name: "see".into(),
            description: "Look at the screen: returns the visible windows, desktop state, and a screenshot. Call this before clicking/typing to find where things are.".into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
        ToolSpec {
            name: "list_windows".into(),
            description: "List the open windows / desktop state (read-only).".into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
        // ---- Read the web ----
        ToolSpec {
            name: "read_webpage".into(),
            description: "Fetch a URL and extract its readable text content.".into(),
            parameters: json!({
                "type": "object",
                "properties": { "url": { "type": "string", "description": "The page URL." } },
                "required": ["url"]
            }),
        },
        // ---- Input (click / type / keys) ----
        ToolSpec {
            name: "click".into(),
            description: "Click at screen coordinates. Use `see` first to find coordinates.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "x": { "type": "integer" },
                    "y": { "type": "integer" },
                    "button": { "type": "string", "enum": ["left", "right"], "description": "Default left." },
                    "double": { "type": "boolean", "description": "Double-click if true." }
                },
                "required": ["x", "y"]
            }),
        },
        ToolSpec {
            name: "move_mouse".into(),
            description: "Move the mouse to screen coordinates.".into(),
            parameters: json!({
                "type": "object",
                "properties": { "x": { "type": "integer" }, "y": { "type": "integer" } },
                "required": ["x", "y"]
            }),
        },
        ToolSpec {
            name: "type_text".into(),
            description: "Type text into the focused field.".into(),
            parameters: json!({
                "type": "object",
                "properties": { "text": { "type": "string" } },
                "required": ["text"]
            }),
        },
        ToolSpec {
            name: "press_key".into(),
            description: "Press a single key (e.g. Enter, Tab, Escape).".into(),
            parameters: json!({
                "type": "object",
                "properties": { "key": { "type": "string" } },
                "required": ["key"]
            }),
        },
        ToolSpec {
            name: "scroll".into(),
            description: "Scroll the active window.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "amount": { "type": "integer", "description": "Number of lines; sign or `direction` sets up/down." },
                    "direction": { "type": "string", "enum": ["up", "down"] }
                },
                "required": ["amount"]
            }),
        },
        ToolSpec {
            name: "hotkey".into(),
            description: "Press a keyboard shortcut, e.g. \"ctrl+c\", \"alt+tab\", \"ctrl+shift+t\".".into(),
            parameters: json!({
                "type": "object",
                "properties": { "keys": { "type": "string" } },
                "required": ["keys"]
            }),
        },
        // ---- Navigate windows ----
        ToolSpec {
            name: "focus_window".into(),
            description: "Bring a window (by title/app name) to the foreground.".into(),
            parameters: json!({
                "type": "object",
                "properties": { "target": { "type": "string" } },
                "required": ["target"]
            }),
        },
        // ---- Long-term memory ----
        ToolSpec {
            name: "remember".into(),
            description: "Save a durable note/fact to remember across sessions (preferences, results, context).".into(),
            parameters: json!({
                "type": "object",
                "properties": { "content": { "type": "string" } },
                "required": ["content"]
            }),
        },
        ToolSpec {
            name: "recall".into(),
            description: "Recall previously saved notes. Optionally filter by a keyword.".into(),
            parameters: json!({
                "type": "object",
                "properties": { "query": { "type": "string", "description": "Optional keyword to match." } }
            }),
        },
        // ---- In-browser navigation (CDP) ----
        ToolSpec {
            name: "browser_open".into(),
            description: "Open a URL in the controlled browser session (persists for follow-up reads/clicks).".into(),
            parameters: json!({
                "type": "object",
                "properties": { "url": { "type": "string" } },
                "required": ["url"]
            }),
        },
        ToolSpec {
            name: "browser_read".into(),
            description: "Read the current browser page's visible text.".into(),
            parameters: json!({ "type": "object", "properties": {} }),
        },
        ToolSpec {
            name: "browser_click".into(),
            description: "Click a link or button in the current browser page by its visible text.".into(),
            parameters: json!({
                "type": "object",
                "properties": { "text": { "type": "string", "description": "Visible text of the link/button." } },
                "required": ["text"]
            }),
        },
        // ---- Scheduled / repeating autonomy ----
        ToolSpec {
            name: "schedule_goal".into(),
            description: "Schedule a goal to run later or repeatedly (the daemon runs it, still approval-gated).".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "goal": { "type": "string", "description": "The goal to run, as you'd type after /do." },
                    "every_secs": { "type": "integer", "description": "Repeat interval in seconds; omit or 0 for one-shot." }
                },
                "required": ["goal"]
            }),
        },
        ToolSpec {
            name: "list_schedules".into(),
            description: "List the currently scheduled goals.".into(),
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
        // Perception — vision_analysis is an observation (no approval); domain
        // must NOT be computer/desktop or it would classify as ComputerControl.
        "see" => Some(("vision_analysis", json!({}), "vision", "see screen".to_string())),
        "list_windows" => Some((
            "os_tool",
            json!({ "action": "list_windows" }),
            "computer",
            "list windows".to_string(),
        )),
        // Read the web.
        "read_webpage" => {
            let url = a.get("url").and_then(|v| v.as_str()).unwrap_or_default();
            Some(("web_extract", json!({ "url": url }), "web", format!("read {url}")))
        }
        // Input.
        "click" => {
            let x = a.get("x").cloned().unwrap_or(json!(0));
            let y = a.get("y").cloned().unwrap_or(json!(0));
            let action = if a.get("double").and_then(|v| v.as_bool()).unwrap_or(false) {
                "double_click"
            } else if a.get("button").and_then(|v| v.as_str()) == Some("right") {
                "right_click"
            } else {
                "click"
            };
            Some((
                "os_tool",
                json!({ "action": action, "x": x, "y": y }),
                "computer",
                format!("{action} at ({x},{y})"),
            ))
        }
        "move_mouse" => {
            let x = a.get("x").cloned().unwrap_or(json!(0));
            let y = a.get("y").cloned().unwrap_or(json!(0));
            Some((
                "os_tool",
                json!({ "action": "move_mouse", "x": x, "y": y }),
                "computer",
                format!("move to ({x},{y})"),
            ))
        }
        "type_text" => {
            let text = a.get("text").and_then(|v| v.as_str()).unwrap_or_default();
            Some((
                "os_tool",
                json!({ "action": "type_text", "text": text }),
                "computer",
                format!("type: {}", short(text, 40)),
            ))
        }
        "press_key" => {
            let key = a.get("key").and_then(|v| v.as_str()).unwrap_or_default();
            Some((
                "os_tool",
                json!({ "action": "press_key", "key": key }),
                "computer",
                format!("press {key}"),
            ))
        }
        "scroll" => Some((
            "os_tool",
            {
                let mut v = a.clone();
                v["action"] = json!("scroll");
                v
            },
            "computer",
            "scroll".to_string(),
        )),
        "hotkey" => {
            let keys = a.get("keys").and_then(|v| v.as_str()).unwrap_or_default();
            Some((
                "os_tool",
                json!({ "action": "hotkey", "keys": keys }),
                "computer",
                format!("hotkey {keys}"),
            ))
        }
        "focus_window" => {
            let target = a.get("target").and_then(|v| v.as_str()).unwrap_or_default();
            Some((
                "os_tool",
                json!({ "action": "focus", "target": target }),
                "computer",
                format!("focus {target}"),
            ))
        }
        // remember / recall are handled internally (see dispatch_tool), not here.
        _ => None,
    }
}

/// Execute one tool call and return an observation string. Computer/file/web
/// actions go through the kernel; the agent's own long-term notes are internal.
async fn dispatch_tool(
    db: &Database,
    run_id: Option<i64>,
    conversation_id: i64,
    call: &ToolCall,
) -> String {
    // Internal tools (the agent's own bookkeeping / reads — not governed OS actions).
    match call.name.as_str() {
        "remember" => return remember_note(db, conversation_id, &call.arguments).await,
        "recall" => return recall_notes(db, &call.arguments).await,
        "schedule_goal" => return schedule_goal_tool(db, conversation_id, &call.arguments).await,
        "list_schedules" => return list_schedules_tool(db).await,
        "browser_read" => return browser_read_internal(conversation_id).await,
        _ => {}
    }

    // Browser actions need a live session id injected before the kernel call.
    if call.name == "browser_open" || call.name == "browser_click" {
        return browser_action(db, run_id, conversation_id, call).await;
    }

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
    // Uses the configured provider — local Ollama by default, cloud (OpenAI/
    // Anthropic) only if selected. The model must support tool-calling.
    // resolve_model falls back to a pulled Ollama model if the configured one
    // isn't installed, so this works out-of-the-box.
    let cfg = providers::resolve_model(&providers::load_config(db).await).await;

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
                return respond(
                    "failed",
                    &format!(
                        "Model error: {e}\n\nIf using local Ollama, make sure it's running (`ollama serve`) and the model supports tool-calling (e.g. qwen2.5, llama3.1, mistral-nemo)."
                    ),
                );
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
            let observation = dispatch_tool(db, run_id, conversation_id, call).await;
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

/// Durable long-term notes for the agent, in a lazily-created SQLite table.
/// This is always available (no optional `memory`/lancedb feature required); the
/// embedding-based semantic store can layer on top later.
async fn ensure_notes_table(db: &Database) {
    let _ = sqlx::query(
        "CREATE TABLE IF NOT EXISTS agent_memories (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            conversation_id INTEGER,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL DEFAULT (datetime('now'))
        )",
    )
    .execute(&db.pool)
    .await;
}

async fn remember_note(db: &Database, conversation_id: i64, args: &Value) -> String {
    let content = args.get("content").and_then(|v| v.as_str()).unwrap_or("").trim();
    if content.is_empty() {
        return "Nothing to remember (empty content).".to_string();
    }
    ensure_notes_table(db).await;
    match sqlx::query("INSERT INTO agent_memories (conversation_id, content) VALUES (?, ?)")
        .bind(conversation_id)
        .bind(content)
        .execute(&db.pool)
        .await
    {
        Ok(_) => format!("Remembered: {}", short(content, 120)),
        Err(e) => format!("Failed to save note: {e}"),
    }
}

async fn recall_notes(db: &Database, args: &Value) -> String {
    ensure_notes_table(db).await;
    let query = args.get("query").and_then(|v| v.as_str()).map(|s| s.trim()).filter(|s| !s.is_empty());
    let rows: Result<Vec<String>, _> = match query {
        Some(q) => {
            sqlx::query_scalar(
                "SELECT content FROM agent_memories WHERE content LIKE ? ORDER BY id DESC LIMIT 20",
            )
            .bind(format!("%{q}%"))
            .fetch_all(&db.pool)
            .await
        }
        None => {
            sqlx::query_scalar("SELECT content FROM agent_memories ORDER BY id DESC LIMIT 20")
                .fetch_all(&db.pool)
                .await
        }
    };
    match rows {
        Ok(notes) if !notes.is_empty() => {
            let list = notes.iter().map(|n| format!("- {n}")).collect::<Vec<_>>().join("\n");
            format!("Saved notes:\n{list}")
        }
        Ok(_) => "No saved notes yet.".to_string(),
        Err(e) => format!("Failed to read notes: {e}"),
    }
}

// ---- Scheduled autonomy tools ----

async fn schedule_goal_tool(db: &Database, conversation_id: i64, args: &Value) -> String {
    let goal = args.get("goal").and_then(|v| v.as_str()).unwrap_or("").trim();
    if goal.is_empty() {
        return "Nothing to schedule (empty goal).".to_string();
    }
    let every = args.get("every_secs").and_then(|v| v.as_i64()).unwrap_or(0).max(0);
    let now = crate::engine::scheduler::now_unix();
    let first = if every > 0 { now + every } else { now };
    match crate::engine::scheduler::add_schedule(db, conversation_id, goal, every, first).await {
        Ok(id) => {
            if every > 0 {
                format!("Scheduled goal #{id}: \"{}\" every {every}s.", short(goal, 80))
            } else {
                format!("Scheduled goal #{id} to run shortly: \"{}\".", short(goal, 80))
            }
        }
        Err(e) => format!("Failed to schedule: {e}"),
    }
}

async fn list_schedules_tool(db: &Database) -> String {
    let list = crate::engine::scheduler::list_schedules(db).await;
    if list.is_empty() {
        return "No scheduled goals.".to_string();
    }
    let now = crate::engine::scheduler::now_unix();
    let lines = list
        .iter()
        .map(|s| {
            let when = (s.next_run_at - now).max(0);
            let repeat = if s.interval_secs > 0 { format!("every {}s", s.interval_secs) } else { "once".into() };
            format!("- #{} in {}s ({}) — {}", s.id, when, repeat, short(&s.goal, 60))
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("Scheduled goals:\n{lines}")
}

// ---- In-browser (CDP) session management ----

/// One controlled browser session per conversation, reused across calls.
static BROWSER_SESSIONS: OnceLock<Mutex<HashMap<i64, i64>>> = OnceLock::new();

async fn ensure_browser_session(conversation_id: i64) -> Result<i64, String> {
    let pool = crate::engine::browser::global::get()
        .cloned()
        .ok_or_else(|| "browser pool not initialized (run the daemon)".to_string())?;
    let map = BROWSER_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut guard = map.lock().await;
    if let Some(&sid) = guard.get(&conversation_id) {
        if pool.session_exists(sid).await {
            return Ok(sid);
        }
    }
    let sid = pool
        .create_session(Some(conversation_id))
        .await
        .map_err(|e| e.to_string())?;
    guard.insert(conversation_id, sid);
    Ok(sid)
}

async fn browser_read_internal(conversation_id: i64) -> String {
    let sid = match ensure_browser_session(conversation_id).await {
        Ok(s) => s,
        Err(e) => return format!("browser unavailable: {e}"),
    };
    match crate::engine::tools::browser::read_page(&json!({ "session_id": sid })).await {
        Ok(mut s) => {
            if s.len() > MAX_OBSERVATION {
                s.truncate(MAX_OBSERVATION);
            }
            s
        }
        Err(e) => format!("browser read failed: {e}"),
    }
}

async fn browser_action(
    db: &Database,
    run_id: Option<i64>,
    conversation_id: i64,
    call: &ToolCall,
) -> String {
    let sid = match ensure_browser_session(conversation_id).await {
        Ok(s) => s,
        Err(e) => return format!("browser unavailable: {e}"),
    };
    let a = &call.arguments;
    let (tool_name, args, step): (&str, Value, String) = match call.name.as_str() {
        "browser_open" => {
            let url = a.get("url").and_then(|v| v.as_str()).unwrap_or_default();
            ("browser_navigate", json!({ "session_id": sid, "url": url }), format!("browser open {url}"))
        }
        "browser_click" => {
            let text = a.get("text").and_then(|v| v.as_str()).unwrap_or_default();
            ("browser_click", json!({ "session_id": sid, "text": text }), format!("browser click '{text}'"))
        }
        _ => return format!("unknown browser action '{}'", call.name),
    };
    let tool = tool_stub(tool_name);
    let outcome = crate::engine::kernel::kernel_execute(
        db,
        run_id,
        "web",
        &step,
        &tool,
        args,
        &[tool_name.to_string()],
    )
    .await;
    match &outcome {
        overseer_kernel::ExecutionOutcome::Executed { output } => {
            let mut o = output.clone();
            if o.len() > MAX_OBSERVATION {
                o.truncate(MAX_OBSERVATION);
            }
            o
        }
        other => other.agent_response(),
    }
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

    #[test]
    fn perception_and_input_tools_are_registered() {
        let names: Vec<String> = tool_catalog().into_iter().map(|t| t.name).collect();
        for expected in [
            "see", "list_windows", "read_webpage", "click", "move_mouse", "type_text",
            "press_key", "scroll", "hotkey", "focus_window", "remember", "recall",
        ] {
            assert!(names.contains(&expected.to_string()), "missing {expected}");
        }
    }

    #[test]
    fn see_routes_to_vision_analysis_observation() {
        // Must be tool "vision_analysis" (an observation tool) with a non-computer
        // domain, so the kernel allows it without approval.
        let call = ToolCall { id: "1".into(), name: "see".into(), arguments: json!({}) };
        let (tool, _args, domain, _) = map_call(&call).unwrap();
        assert_eq!(tool, "vision_analysis");
        assert_ne!(domain, "computer");
        assert_ne!(domain, "desktop");
    }

    #[test]
    fn click_maps_button_and_double_to_os_tool_actions() {
        let right = ToolCall { id: "1".into(), name: "click".into(), arguments: json!({"x":10,"y":20,"button":"right"}) };
        let (_, args, domain, _) = map_call(&right).unwrap();
        assert_eq!(args["action"], "right_click");
        assert_eq!(args["x"], 10);
        assert_eq!(domain, "computer");

        let dbl = ToolCall { id: "1".into(), name: "click".into(), arguments: json!({"x":1,"y":2,"double":true}) };
        assert_eq!(map_call(&dbl).unwrap().1["action"], "double_click");
    }

    #[test]
    fn type_text_maps_to_os_tool_type_text() {
        let call = ToolCall { id: "1".into(), name: "type_text".into(), arguments: json!({"text":"hello"}) };
        let (tool, args, _, _) = map_call(&call).unwrap();
        assert_eq!(tool, "os_tool");
        assert_eq!(args["action"], "type_text");
        assert_eq!(args["text"], "hello");
    }

    #[test]
    fn browser_and_schedule_tools_registered() {
        let names: Vec<String> = tool_catalog().into_iter().map(|t| t.name).collect();
        for expected in ["browser_open", "browser_read", "browser_click", "schedule_goal", "list_schedules"] {
            assert!(names.contains(&expected.to_string()), "missing {expected}");
        }
    }

    #[tokio::test]
    async fn schedule_then_list_roundtrips() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "t").await.unwrap();
        let out = schedule_goal_tool(&db, conv, &json!({ "goal": "brief me", "every_secs": 3600 })).await;
        assert!(out.contains("Scheduled goal"));
        let listed = list_schedules_tool(&db).await;
        assert!(listed.contains("brief me"));
    }

    #[tokio::test]
    async fn remember_then_recall_roundtrips() {
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "t").await.unwrap();
        let saved = remember_note(&db, conv, &json!({ "content": "user prefers dark mode" })).await;
        assert!(saved.contains("Remembered"));
        let all = recall_notes(&db, &json!({})).await;
        assert!(all.contains("dark mode"));
        let filtered = recall_notes(&db, &json!({ "query": "dark" })).await;
        assert!(filtered.contains("dark mode"));
        let miss = recall_notes(&db, &json!({ "query": "zzz-nope" })).await;
        assert!(miss.contains("No saved notes"));
    }

    #[tokio::test]
    async fn fails_gracefully_when_model_backend_unreachable() {
        // Point at an unreachable cloud endpoint so the very first model call
        // errors immediately: the loop must return a structured 'failed' response
        // (not panic, not hang). Using a bogus cloud base_url keeps this
        // deterministic regardless of whether local Ollama happens to be running.
        let db = crate::db::Database::new_in_memory().await.unwrap();
        let conv = crate::backend::add_conversation(&db, "t").await.unwrap();
        let _ = crate::backend::set_setting(&db, "llm_provider", "openai").await;
        let _ = crate::backend::set_setting(&db, "openai_api_key", "sk-test").await;
        let _ = crate::backend::set_setting(&db, "openai_base_url", "http://127.0.0.1:1").await;

        let out = run_agent(&db, conv, "list my home dir", None).await;
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["status"], "failed");
        assert!(out.contains("Model error"), "should surface the backend error");
    }
}
