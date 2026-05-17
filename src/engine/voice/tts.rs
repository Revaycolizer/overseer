use std::process::Command;

pub fn speak_locally(text: &str) {
    let clean_text = text
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace() || *c == '.' || *c == ',' || *c == '!' || *c == '?')
        .collect::<String>();
    
    println!("🔊 Local TTS Synthesis: {}", clean_text);

    std::thread::spawn(move || {
        // Try calling piper if installed
        if Command::new("piper").arg("--version").output().is_ok() {
            let model_path = "assets/voice/models/en_US-lessac-medium.onnx";
            if std::path::Path::new(model_path).exists() {
                let pipe_cmd = format!("echo \"{}\" | piper --model {} --output_file /tmp/overseer_tts.wav && aplay /tmp/overseer_tts.wav", clean_text, model_path);
                let _ = Command::new("sh").arg("-c").arg(&pipe_cmd).output();
                return;
            }
        }
        
        // Fallback to spd-say
        if Command::new("spd-say").arg("--version").output().is_ok() {
            let _ = Command::new("spd-say")
                .arg("-t")
                .arg("female1")
                .arg(&clean_text)
                .output();
            return;
        }

        // Fallback to espeak
        if Command::new("espeak").arg("--version").output().is_ok() {
            let _ = Command::new("espeak")
                .arg(&clean_text)
                .output();
        }
    });
}
