use std::process::Command;
use std::sync::atomic::{AtomicI32, Ordering};

static TTS_SPEAKING_COUNT: AtomicI32 = AtomicI32::new(0);

pub fn is_tts_speaking() -> bool {
    TTS_SPEAKING_COUNT.load(Ordering::SeqCst) > 0
}

pub fn speak_locally(text: &str) {
    let clean_text = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == ',' || *c == '!' || *c == '?')
        .collect::<String>();
    
    println!("🔊 Local TTS Synthesis: {}", clean_text);

    TTS_SPEAKING_COUNT.fetch_add(1, Ordering::SeqCst);
    std::thread::spawn(move || {
        let temp_wav = std::env::temp_dir().join("overseer_tts.wav");
        let temp_wav_str = temp_wav.to_string_lossy().to_string();

        // Helper macro/closure to safely decrement before return
        let mut play_finished = || {
            TTS_SPEAKING_COUNT.fetch_sub(1, Ordering::SeqCst);
        };

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
                            play_finished();
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
            play_finished();
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
            play_finished();
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
                play_finished();
                return;
            }

            if Command::new("espeak").arg("--version").output().is_ok() {
                let _ = Command::new("espeak")
                    .arg(&clean_text)
                    .output();
            }
        }
        play_finished();
    });
}
