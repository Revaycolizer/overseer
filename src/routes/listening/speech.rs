use dioxus::prelude::*;
use dioxus::prelude::document::eval;
use crate::state::GlobalState;

pub fn use_speech_recognition(state: Signal<GlobalState>) {
    use_effect(move || {
        let mut state = state.clone();
        spawn(async move {
            if state.read().voice_transcript.read().is_some() {
                return;
            }
            
            // Wait 1.5s for initial greeting TTS to complete
            tokio::time::sleep(std::time::Duration::from_millis(1500)).await;
            
            let eval_handle = eval(r#"
                return new Promise((resolve, reject) => {
                    const SpeechRec = window.SpeechRecognition || window.webkitSpeechRecognition;
                    if (!SpeechRec) {
                        reject("Web Speech API not supported");
                        return;
                    }
                    const rec = new SpeechRec();
                    rec.lang = 'en-US';
                    rec.interimResults = false;
                    rec.continuous = false;
                    rec.start();
                    rec.onresult = (e) => {
                        resolve(e.results[0][0].transcript);
                    };
                    rec.onerror = (e) => {
                        reject(e.error);
                    };
                });
            "#);

            if let Ok(value) = eval_handle.await {
                if let Some(transcript_text) = value.as_str() {
                    let cmd = transcript_text.trim().to_string();
                    if !cmd.is_empty() {
                        println!("🎙️ Web Speech API transcribed: \"{}\"", cmd);
                        state.write().voice_transcript.set(Some(cmd.clone()));
                        
                        let db = state.peek().db.clone();
                        let agent = state.peek().agent_system.clone();
                        spawn(async move {
                            match crate::engine::voice::pipeline::execute_voice_command(&db, &agent, &cmd).await {
                                Ok(res) => {
                                    state.write().voice_result.set(Some(res));
                                }
                                Err(e) => {
                                    state.write().voice_result.set(Some(format!("Execution Error: {}", e)));
                                }
                            }
                        });
                    }
                }
            }
        });
    });
}
