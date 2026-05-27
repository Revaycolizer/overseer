use dioxus::prelude::*;
use dioxus::prelude::document::eval;
use crate::state::GlobalState;

pub fn use_speech_recognition(state: Signal<GlobalState>) {
    let is_listening_sig = state.peek().is_listening;
    let is_inhibited_sig = state.peek().is_inhibited;
    let voice_transcript_sig = state.peek().voice_transcript;
    let voice_result_sig = state.peek().voice_result;
    let db = state.peek().db.clone();
    let agent = state.peek().agent_system.clone();

    use_effect(move || {
        let is_listening = *is_listening_sig.read();
        if !is_listening {
            return;
        }
        
        let db = db.clone();
        let agent = agent.clone();
        let mut is_listening_sig = is_listening_sig.clone();
        let mut is_inhibited_sig = is_inhibited_sig.clone();
        let mut voice_transcript_sig = voice_transcript_sig.clone();
        let mut voice_result_sig = voice_result_sig.clone();

        spawn(async move {
            if voice_transcript_sig.read().is_some() {
                return;
            }
            
            // Wait 200ms for initial greeting TTS to start clearing audio channel
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
            
            let mut eval_handle = eval(r#"
                const SpeechRec = window.SpeechRecognition || window.webkitSpeechRecognition;
                if (!SpeechRec) {
                    dioxus.send({ type: "error", message: "Web Speech API not supported" });
                    return;
                }
                const rec = new SpeechRec();
                rec.lang = 'en-US';
                rec.interimResults = true;
                rec.continuous = false;
                rec.start();
                
                rec.onresult = (e) => {
                    let interim = "";
                    let finalResult = "";
                    for (let i = e.resultIndex; i < e.results.length; ++i) {
                        const text = e.results[i][0].transcript;
                        if (e.results[i].isFinal) {
                            finalResult += text;
                        } else {
                            interim += text;
                        }
                    }
                    if (finalResult.length > 0) {
                        dioxus.send({ type: "final", text: finalResult });
                    } else if (interim.length > 0) {
                        dioxus.send({ type: "interim", text: interim });
                    }
                };
                
                rec.onerror = (e) => {
                    dioxus.send({ type: "error", message: e.error });
                };
                
                rec.onend = () => {
                    dioxus.send({ type: "end" });
                };
            "#);

            let mut final_received = false;
            while let Ok(msg) = eval_handle.recv::<serde_json::Value>().await {
                if let Some(msg_type) = msg.get("type").and_then(|v| v.as_str()) {
                    match msg_type {
                        "interim" => {
                            if !final_received {
                                if let Some(text) = msg.get("text").and_then(|v| v.as_str()) {
                                    voice_transcript_sig.set(Some(text.to_string()));
                                }
                            }
                        }
                        "final" => {
                            if let Some(text) = msg.get("text").and_then(|v| v.as_str()) {
                                let cmd = text.trim().to_string();
                                if !cmd.is_empty() {
                                    final_received = true;
                                    println!("🎙️ Web Speech API transcribed: \"{}\"", cmd);
                                    voice_transcript_sig.set(Some(cmd.clone()));
                                    crate::engine::voice::earcons::play_thinking_hum();
                                    
                                    match crate::engine::voice::pipeline::execute_voice_command(&db, &agent, &cmd).await {
                                        Ok(res) => {
                                            voice_result_sig.set(Some(res));
                                        }
                                        Err(e) => {
                                            voice_result_sig.set(Some(format!("Execution Error: {}", e)));
                                            crate::engine::voice::earcons::play_blocked_beep();
                                            crate::engine::voice::tts::speak_locally("Task execution failed.");
                                        }
                                    }
                                    
                                    is_listening_sig.set(false);
                                    is_inhibited_sig.set(false);
                                    break;
                                }
                            }
                        }
                        "error" => {
                            eprintln!("🎙️ Speech Recognition Error: {:?}. Running local fallback...", msg.get("message"));
                            let db = db.clone();
                            let agent = agent.clone();
                            let voice_transcript_sig = voice_transcript_sig.clone();
                            let voice_result_sig = voice_result_sig.clone();
                            let is_listening_sig = is_listening_sig.clone();
                            let is_inhibited_sig = is_inhibited_sig.clone();
                            spawn(async move {
                                run_local_stt_fallback(db, agent, voice_transcript_sig, voice_result_sig, is_listening_sig, is_inhibited_sig).await;
                            });
                            break;
                        }
                        "end" => {
                            if !final_received {
                                println!("🎙️ Speech Recognition ended without result. Running local fallback...");
                                let db = db.clone();
                                let agent = agent.clone();
                                let voice_transcript_sig = voice_transcript_sig.clone();
                                let voice_result_sig = voice_result_sig.clone();
                                let is_listening_sig = is_listening_sig.clone();
                                let is_inhibited_sig = is_inhibited_sig.clone();
                                spawn(async move {
                                    run_local_stt_fallback(db, agent, voice_transcript_sig, voice_result_sig, is_listening_sig, is_inhibited_sig).await;
                                });
                            }
                            break;
                        }
                        _ => {}
                    }
                }
            }
        });
    });
}

async fn run_local_stt_fallback(
    db: std::sync::Arc<crate::db::Database>,
    agent: std::sync::Arc<crate::engine::system::AgentSystem>,
    mut voice_transcript: Signal<Option<String>>,
    mut voice_result: Signal<Option<String>>,
    mut is_listening: Signal<bool>,
    mut is_inhibited: Signal<bool>,
) {
    let vad_threshold = crate::engine::voice::wake_word::get_vad_threshold();
    match crate::engine::voice::local_stt::record_and_transcribe(vad_threshold).await {
        Ok(cmd) => {
            println!("🎙️ Local offline STT transcribed: \"{}\"", cmd);
            voice_transcript.set(Some(cmd.clone()));
            crate::engine::voice::earcons::play_thinking_hum();
            match crate::engine::voice::pipeline::execute_voice_command(&db, &agent, &cmd).await {
                Ok(res) => {
                    voice_result.set(Some(res));
                }
                Err(e) => {
                    voice_result.set(Some(format!("Execution Error: {}", e)));
                    crate::engine::voice::earcons::play_blocked_beep();
                    crate::engine::voice::tts::speak_locally("Task execution failed.");
                }
            }
        }
        Err(err) => {
            eprintln!("🎙️ Local STT fallback failed: {}", err);
            crate::engine::voice::earcons::play_blocked_beep();
            crate::engine::voice::tts::speak_locally("Speech recognition failed.");
        }
    }
    is_listening.set(false);
    is_inhibited.set(false);
}
