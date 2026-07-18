use std::process::Command;
use hound::{WavSpec, WavWriter, SampleFormat};

pub fn transcribe_samples_sync(recorded_data: &[f32]) -> Result<String, String> {
    if recorded_data.is_empty() { return Err("No audio".to_string()); }
    let temp_wav = std::env::temp_dir().join("overseer_speech_stt.wav");
    let temp_wav_str = temp_wav.to_string_lossy().to_string();
    let spec = WavSpec { channels: 1, sample_rate: 16000, bits_per_sample: 32, sample_format: SampleFormat::Float };
    let mut w = WavWriter::create(&temp_wav, spec).map_err(|e| e.to_string())?;
    for &val in recorded_data { let _ = w.write_sample(val); }
    w.finalize().map_err(|e| e.to_string())?;

    let mut vosk_cmd = "vosk-transcriber".to_string();
    if !Command::new(&vosk_cmd).arg("--version").output().is_ok() {
        if let Some(home) = dirs::home_dir() {
            let local_bin = home.join(".local").join("bin").join("vosk-transcriber");
            if local_bin.exists() {
                vosk_cmd = local_bin.to_string_lossy().to_string();
            }
        }
    }

    if Command::new(&vosk_cmd).arg("--version").output().is_ok() {
        if let Ok(out) = Command::new(&vosk_cmd).args(["-i", &temp_wav_str, "-o", "/dev/stdout"]).output() {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !text.is_empty() { return Ok(text); }
            }
        }
    }
    if Command::new("whisper").arg("--version").output().is_ok() {
        let temp_dir = std::env::temp_dir();
        if let Ok(out) = Command::new("whisper").args([&temp_wav_str, "--model", "tiny.en", "--output_dir", &temp_dir.to_string_lossy(), "--output_format", "txt"]).output() {
            if out.status.success() {
                let txt_path = temp_dir.join("overseer_speech_stt.txt");
                if txt_path.exists() {
                    if let Ok(text) = std::fs::read_to_string(&txt_path) {
                        let _ = std::fs::remove_file(txt_path);
                        let clean = text.trim().to_string();
                        if !clean.is_empty() { return Ok(clean); }
                    }
                }
            }
        }
    }

    let py_script = r#"
import sys, os
wav_path = sys.argv[1]
try:
    import speech_recognition as sr
    r = sr.Recognizer()
    with sr.AudioFile(wav_path) as src:
        print(r.recognize_sphinx(r.record(src)))
        sys.exit(0)
except Exception: pass
try:
    from vosk import Model, KaldiRecognizer
    import wave, json
    wf = wave.open(wav_path, "rb")
    m_dir = os.path.expanduser("~/.cache/vosk/vosk-model-small-en-us-0.15")
    if os.path.exists(m_dir):
        rec = KaldiRecognizer(Model(m_dir), wf.getframerate())
        results = []
        while True:
            data = wf.readframes(4000)
            if len(data) == 0: break
            if rec.AcceptWaveform(data):
                res = json.loads(rec.Result())
                if res.get("text"): results.append(res["text"])
        res = json.loads(rec.FinalResult())
        if res.get("text"): results.append(res["text"])
        text = " ".join(results).strip()
        if text:
            print(text)
            sys.exit(0)
except Exception: pass
sys.exit(1)
"#;
    let py_path = std::env::temp_dir().join("overseer_transcribe.py");
    let _ = std::fs::write(&py_path, py_script);
    let py_cmd = if cfg!(target_os = "windows") { "python" } else { "python3" };
    let output = Command::new(py_cmd).arg(&py_path).arg(&temp_wav_str).output();
    let _ = std::fs::remove_file(py_path);

    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !text.is_empty() { return Ok(text); }
        }
    }
    Err("All offline STT fallbacks failed.".to_string())
}
