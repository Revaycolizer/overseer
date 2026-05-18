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
            
            // Wait 600ms for initial greeting TTS to complete (2.5x faster startup than 1500ms!)
            tokio::time::sleep(std::time::Duration::from_millis(600)).await;
            
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
                                    
                                    crate::engine::voice::tts::speak_locally(&format!("Executing: {}", cmd));

                                    match crate::engine::voice::pipeline::execute_voice_command(&db, &agent, &cmd).await {
                                        Ok(res) => {
                                            voice_result_sig.set(Some(res));
                                            crate::engine::voice::tts::speak_locally("Task completed successfully.");
                                        }
                                        Err(e) => {
                                            voice_result_sig.set(Some(format!("Execution Error: {}", e)));
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
                            eprintln!("🎙️ Speech Recognition Error: {:?}", msg.get("message"));
                            is_listening_sig.set(false);
                            is_inhibited_sig.set(false);
                            break;
                        }
                        "end" => {
                            // Only close if we didn't receive a final command
                            if !final_received {
                                is_listening_sig.set(false);
                                is_inhibited_sig.set(false);
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
