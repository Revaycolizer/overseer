use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::db::Database;
use crate::engine::voice::stt_optimizer::SpeechOptimizer;

pub struct AgentOrchestrator {
    pub is_tts_speaking: Arc<AtomicBool>,
    pub db_conn: Arc<Database>,
    pub speech_engine: SpeechOptimizer,
}

#[derive(Debug)]
pub enum Action {
    LaunchApp(String),
    ExecuteShell(String),
    CaptureScreenVerify,
    SpeakFeedback(String),
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
        match action {
            Action::LaunchApp(app) => { std::process::Command::new(app).spawn()?; }
            Action::ExecuteShell(cmd) => {
                #[cfg(target_os = "linux")]
                std::process::Command::new("sh").arg("-c").arg(&cmd).output()?;
                #[cfg(target_os = "windows")]
                std::process::Command::new("powershell").arg("-Command").arg(&cmd).output()?;
            }
            Action::CaptureScreenVerify => {
                let _ = crate::engine::tools::native_os_tool(&serde_json::json!({ "action": "screenshot" })).await;
            }
            Action::SpeakFeedback(text) => {
                self.is_tts_speaking.store(true, Ordering::SeqCst);
                crate::engine::voice::tts::speak_locally(&text);
                self.is_tts_speaking.store(false, Ordering::SeqCst);
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
        if lower.contains("firefox") && context.contains("Firefox") { return Ok(None); }
        if lower.contains("firefox") { return Ok(Some(Action::LaunchApp("firefox".to_string()))); }
        if lower.contains("screenshot") { return Ok(Some(Action::CaptureScreenVerify)); }

        use sqlx::Row;
        let rows = sqlx::query("SELECT name FROM tools").fetch_all(&self.db_conn.pool).await.unwrap_or_default();
        for r in rows {
            let name: String = r.get("name");
            if lower.contains(&name.to_lowercase()) {
                return Ok(Some(Action::ExecuteShell(format!("echo 'Executing {}'", name))));
            }
        }
        Ok(None)
    }
}
