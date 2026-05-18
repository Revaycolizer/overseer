use std::process::Command;

pub fn speak_locally(text: &str) {
    let clean_text = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == ',' || *c == '!' || *c == '?')
        .collect::<String>();
    
    println!("🔊 Local TTS Synthesis: {}", clean_text);

    std::thread::spawn(move || {
        let temp_wav = std::env::temp_dir().join("overseer_tts.wav");
        let temp_wav_str = temp_wav.to_string_lossy().to_string();

        // 1. Try calling piper if installed (Cross-Platform direct stdin implementation)
        if Command::new("piper").arg("--version").output().is_ok() {
            let model_path = "assets/voice/models/en_US-lessac-medium.onnx";
            if std::path::Path::new(model_path).exists() {
                use std::io::Write;
                if let Ok(mut child) = Command::new("piper")
                    .arg("--model")
                    .arg(model_path)
                    .arg("--output_file")
                    .arg(&temp_wav_str)
                    .stdin(std::process::Stdio::piped())
                    .spawn()
                {
                    if let Some(mut stdin) = child.stdin.take() {
                        let _ = writeln!(stdin, "{}", clean_text);
                    }
                    if let Ok(status) = child.wait() {
                        if status.success() {
                            // Play the audio file cross-platform
                            #[cfg(target_os = "linux")]
                            {
                                let _ = Command::new("aplay").arg(&temp_wav_str).output();
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = Command::new("afplay").arg(&temp_wav_str).output();
                            }
                            #[cfg(target_os = "windows")]
                            {
                                let play_script = format!(
                                    "(New-Object System.Media.SoundPlayer '{}').PlaySync()",
                                    temp_wav_str
                                );
                                let _ = Command::new("powershell")
                                    .arg("-Command")
                                    .arg(&play_script)
                                    .output();
                            }
                            return;
                        }
                    }
                }
            }
        }

        // 2. Native OS built-in high-quality speech engines (Zero-dependency fallbacks)
        #[cfg(target_os = "macos")]
        {
            let _ = Command::new("say").arg(&clean_text).output();
            return;
        }

        #[cfg(target_os = "windows")]
        {
            let ps_cmd = format!(
                "Add-Type -AssemblyName System.Speech; (New-Object System.Speech.Synthesis.SpeechSynthesizer).Speak('{}')",
                clean_text.replace("'", "''")
            );
            let _ = Command::new("powershell")
                .arg("-Command")
                .arg(&ps_cmd)
                .output();
            return;
        }

        // 3. Linux command line fallbacks
        #[cfg(target_os = "linux")]
        {
            if Command::new("spd-say").arg("--version").output().is_ok() {
                let _ = Command::new("spd-say")
                    .arg("-t")
                    .arg("female1")
                    .arg(&clean_text)
                    .output();
                return;
            }

            if Command::new("espeak").arg("--version").output().is_ok() {
                let _ = Command::new("espeak")
                    .arg(&clean_text)
                    .output();
            }
        }
    });
}
