use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::db::Database;
use crate::engine::voice::stt_optimizer::SpeechOptimizer;

pub struct AgentOrchestrator {
    pub is_tts_speaking: Arc<AtomicBool>,
    pub db_conn: Arc<Database>,
    pub speech_engine: SpeechOptimizer,
}

#[derive(Debug, Clone)]
pub enum Action {
    LaunchApp(String),
    ExecuteShell(String),
    CaptureScreenVerify,
    SpeakFeedback(String),
    ExecuteDynamicTool(crate::engine::utils::discovery::ToolInfo, String),
}

impl AgentOrchestrator {
    pub fn new(db: Arc<Database>, speech_engine: SpeechOptimizer) -> Self {
        Self {
            is_tts_speaking: Arc::new(AtomicBool::new(false)),
            db_conn: db,
            speech_engine,
        }
    }

    pub async fn process_voice_input(&self, raw_audio: &[f32]) -> Result<(), Box<dyn std::error::Error>> {
        if self.is_tts_speaking.load(Ordering::SeqCst) || crate::engine::voice::tts::is_tts_speaking() {
            return Ok(());
        }
        crate::engine::voice::earcons::play_listening_chime();
        let transcript = self.speech_engine.transcribe_stream(raw_audio);
        if transcript.trim().is_empty() { return Ok(()); }

        if let Some(fast_action) = self.match_fast_path(&transcript) {
            self.execute_action(fast_action).await?;
            return Ok(());
        }
        self.execute_agentic_loop(&transcript).await?;
        Ok(())
    }

    fn match_fast_path(&self, query: &str) -> Option<Action> {
        let lower = query.to_lowercase();
        if lower.contains("open firefox") {
            Some(Action::LaunchApp("firefox".to_string()))
        } else if lower.contains("take a screenshot") {
            Some(Action::CaptureScreenVerify)
        } else {
            None
        }
    }

    async fn execute_agentic_loop(&self, goal: &str) -> Result<(), Box<dyn std::error::Error>> {
        let mut steps = 0;
        while steps < 5 {
            let context = self.capture_accessibility_and_pixels().await;
            match self.resolve_next_agent_step(goal, &context).await? {
                Some(action) => {
                    self.execute_action(action).await?;
                    steps += 1;
                }
                None => break,
            }
        }
        Ok(())
    }

    async fn execute_action(&self, action: Action) -> Result<(), Box<dyn std::error::Error>> {
        use overseer_kernel::{ActionKind, PolicyDecision};
        match action {
            Action::LaunchApp(app) => {
                // Launching an app is a computer action; it must pass kernel
                // policy. The voice loop cannot wait for approval, so it only
                // proceeds on a deterministic Allow.
                let args = serde_json::json!({ "action": "launch", "target": app });
                match crate::engine::kernel::evaluate_named(
                    &self.db_conn, "computer", ActionKind::ComputerControl, "app_launcher", &args, &["app_launcher".to_string()],
                )
                .await
                {
                    PolicyDecision::Allow => { std::process::Command::new(&app).spawn()?; }
                    _ => {
                        println!("🛡️ Kernel blocked voice LaunchApp('{}') — not authorized.", app);
                    }
                }
            }
            Action::ExecuteShell(cmd) => {
                // No `sh -c` / `powershell -Command`. Shell commands are gated by
                // the kernel and, when permitted, run as an executable + argument
                // array (never a shell-interpreted string).
                let args = serde_json::json!({ "action": "shell", "command": cmd });
                match crate::engine::kernel::evaluate_named(
                    &self.db_conn, "computer", ActionKind::ShellCommand, "shell", &args, &[],
                )
                .await
                {
                    PolicyDecision::Allow => {
                        let mut parts = cmd.split_whitespace();
                        if let Some(program) = parts.next() {
                            let rest: Vec<&str> = parts.collect();
                            let _ = std::process::Command::new(program).args(&rest).output()?;
                        }
                    }
                    _ => {
                        println!("🛡️ Kernel blocked voice shell command — not authorized.");
                    }
                }
            }
            Action::CaptureScreenVerify => {
                let args = serde_json::json!({ "action": "screenshot" });
                if matches!(
                    crate::engine::kernel::evaluate_named(
                        &self.db_conn, "computer", ActionKind::ComputerControl, "os_tool", &args, &["os_tool".to_string()],
                    )
                    .await,
                    PolicyDecision::Allow
                ) {
                    let _ = crate::engine::tools::native_os_tool(&args).await;
                } else {
                    println!("🛡️ Kernel blocked voice screenshot — not authorized.");
                }
            }
            Action::SpeakFeedback(text) => {
                self.is_tts_speaking.store(true, Ordering::SeqCst);
                crate::engine::voice::tts::speak_locally(&text);
                self.is_tts_speaking.store(false, Ordering::SeqCst);
            }
            Action::ExecuteDynamicTool(tool, goal) => {
                let args = serde_json::json!({
                    "tool_name": tool.name.clone(),
                    "input": goal
                });
                let kind = crate::engine::kernel::classify_action_kind(&tool, "computer");
                match crate::engine::kernel::evaluate_named(
                    &self.db_conn, "computer", kind, &tool.name, &args, &[tool.name.clone()],
                )
                .await
                {
                    PolicyDecision::Allow => {
                        match crate::engine::tools::call_dynamic_tool(&tool, args).await {
                            Ok(output) => {
                                println!("Dynamic tool output: {}", output);
                                self.is_tts_speaking.store(true, Ordering::SeqCst);
                                crate::engine::voice::tts::speak_locally(&format!("Executed tool {}", tool.name));
                                self.is_tts_speaking.store(false, Ordering::SeqCst);
                            }
                            Err(e) => {
                                println!("Dynamic tool execution error: {}", e);
                            }
                        }
                    }
                    _ => {
                        println!("🛡️ Kernel blocked voice tool '{}' — requires approval or not authorized.", tool.name);
                        self.is_tts_speaking.store(true, Ordering::SeqCst);
                        crate::engine::voice::tts::speak_locally("That action requires approval and can't be run by voice.");
                        self.is_tts_speaking.store(false, Ordering::SeqCst);
                    }
                }
            }
        }
        Ok(())
    }

    async fn capture_accessibility_and_pixels(&self) -> String {
        let acc = crate::engine::vision::accessibility::capture_accessibility_tree().await.unwrap_or_default();
        format!("Screen: Active Window [Editor]. Accessibility: {}", acc)
    }

    async fn resolve_next_agent_step(&self, goal: &str, context: &str) -> Result<Option<Action>, Box<dyn std::error::Error>> {
        let lower = goal.to_lowercase();

        // 1. Query database-defined tools first to execute them dynamically
        let tools = crate::engine::utils::discovery::get_tools(&self.db_conn).await;
        for tool in tools {
            if lower.contains(&tool.name.to_lowercase()) {
                return Ok(Some(Action::ExecuteDynamicTool(tool, goal.to_string())));
            }
        }

        // 2. Fall back to local fast-path heuristics for OS actions
        if lower.contains("firefox") && context.contains("Firefox") { return Ok(None); }
        if lower.contains("firefox") { return Ok(Some(Action::LaunchApp("firefox".to_string()))); }
        if lower.contains("screenshot") { return Ok(Some(Action::CaptureScreenVerify)); }

        Ok(None)
    }
}
