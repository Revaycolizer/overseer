use hound::{SampleFormat, WavSpec, WavWriter};
use std::path::{Path, PathBuf};
use std::process::Command;

fn is_executable(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        return path
            .metadata()
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false);
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn find_in_dirs(name: &str, dirs: impl IntoIterator<Item = PathBuf>) -> Option<PathBuf> {
    for dir in dirs {
        let path = dir.join(name);
        if is_executable(&path) {
            return Some(path);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{name}.exe"));
            if is_executable(&exe) {
                return Some(exe);
            }
        }
    }
    None
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let mut paths: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".local/bin"));
    }
    paths.extend(["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"].map(PathBuf::from));
    find_in_dirs(name, paths)
}

pub(super) fn whisper_model_path() -> PathBuf {
    std::env::var_os("OVERSEER_WHISPER_MODEL")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            crate::db::get_overseer_data_dir().join("assets/voice/models/ggml-tiny.en.bin")
        })
}

fn write_audio(path: &Path, samples: &[f32]) -> Result<(), String> {
    // PCM16 works with whisper.cpp, Vosk and Python's wave/Sphinx readers.
    let spec = WavSpec {
        channels: 1,
        sample_rate: 16000,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    let mut writer = WavWriter::create(path, spec).map_err(|e| e.to_string())?;
    for &sample in samples {
        let value = if sample.is_finite() {
            sample.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        writer
            .write_sample((value * i16::MAX as f32).round() as i16)
            .map_err(|e| e.to_string())?;
    }
    writer.finalize().map_err(|e| e.to_string())
}

fn read_transcript(output: &std::process::Output, path: &Path) -> Result<String, String> {
    if !output.status.success() {
        return Err(format!(
            "transcriber exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    std::fs::read_to_string(path)
        .map(|text| text.trim().to_string())
        .map_err(|e| format!("transcriber did not write a transcript: {e}"))
}

pub fn transcribe_samples_sync(recorded_data: &[f32]) -> Result<String, String> {
    if recorded_data.is_empty() {
        return Err("No audio".to_string());
    }
    // Separate requests cannot overwrite audio/transcripts or read stale results.
    let scratch = tempfile::Builder::new()
        .prefix("overseer-stt-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let temp_wav = scratch.path().join("speech.wav");
    write_audio(&temp_wav, recorded_data)?;
    let mut failures = Vec::new();

    if let Some(cmd) = find_executable("vosk-transcriber") {
        match Command::new(cmd)
            .arg("-i")
            .arg(&temp_wav)
            .args(["-o", "/dev/stdout"])
            .output()
        {
            Ok(out) if out.status.success() => {
                let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !text.is_empty() {
                    return Ok(text);
                }
            }
            Ok(_) => failures.push("Vosk transcription failed".to_string()),
            Err(e) => failures.push(format!("Vosk: {e}")),
        }
    }

    if let Some(cmd) = find_executable("whisper-cli") {
        let model = whisper_model_path();
        if model.is_file() {
            let output_base = scratch.path().join("transcript");
            match Command::new(cmd)
                .arg("-m")
                .arg(&model)
                .arg("-f")
                .arg(&temp_wav)
                .args(["-otxt", "-of"])
                .arg(&output_base)
                .args(["-l", "en", "-nt"])
                .output()
            {
                Ok(out) => match read_transcript(&out, &output_base.with_extension("txt")) {
                    Ok(text) => return Ok(text), // Empty is valid silence; do not retry it.
                    Err(e) => failures.push(format!("whisper.cpp: {e}")),
                },
                Err(e) => failures.push(format!("whisper.cpp: {e}")),
            }
        } else {
            failures.push(format!("Whisper model is missing: {}", model.display()));
        }
    }

    // The Python Whisper CLI is optional. Do not try spawning an absent binary.
    if let Some(cmd) = find_executable("whisper") {
        match Command::new(cmd)
            .arg(&temp_wav)
            .args(["--model", "tiny.en", "--output_dir"])
            .arg(scratch.path())
            .args(["--output_format", "txt", "--fp16", "False"])
            .output()
        {
            Ok(out) => match read_transcript(&out, &scratch.path().join("speech.txt")) {
                Ok(text) => return Ok(text),
                Err(e) => failures.push(format!("Python Whisper: {e}")),
            },
            Err(e) => failures.push(format!("Python Whisper: {e}")),
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
    let py_cmd = if cfg!(target_os = "windows") {
        "python"
    } else {
        "python3"
    };
    if let Some(cmd) = find_executable(py_cmd) {
        if let Ok(out) = Command::new(cmd)
            .arg("-c")
            .arg(py_script)
            .arg(&temp_wav)
            .output()
        {
            if out.status.success() {
                return Ok(String::from_utf8_lossy(&out.stdout).trim().to_string());
            }
        }
    }
    let details = if failures.is_empty() {
        String::new()
    } else {
        format!(" {}.", failures.join("; "))
    };
    Err(format!("Offline speech recognition is unavailable.{details} On macOS, run scripts/setup-local-voice.sh, then restart Overseer. Other platforms need whisper-cli and a GGML model configured with OVERSEER_WHISPER_MODEL, or the Python whisper CLI."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_is_pcm16_and_clamped_for_all_backends() {
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().join("speech.wav");
        write_audio(&path, &[-2.0, 0.5, 2.0, f32::NAN]).unwrap();
        let mut wav = hound::WavReader::open(path).unwrap();
        assert_eq!(wav.spec().sample_format, SampleFormat::Int);
        assert_eq!(wav.spec().sample_rate, 16000);
        assert_eq!(wav.spec().channels, 1);
        assert_eq!(
            wav.samples::<i16>().map(Result::unwrap).collect::<Vec<_>>(),
            [-32767, 16384, 32767, 0]
        );
    }

    #[test]
    fn empty_audio_does_not_invoke_backends() {
        assert_eq!(transcribe_samples_sync(&[]), Err("No audio".into()));
    }

    #[cfg(unix)]
    #[test]
    fn resolution_requires_executable_and_searches_later_directories() {
        use std::os::unix::fs::PermissionsExt;
        let scratch = tempfile::tempdir().unwrap();
        let first = scratch.path().join("first");
        let second = scratch.path().join("second");
        std::fs::create_dir_all(&first).unwrap();
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(first.join("whisper-cli"), "").unwrap();
        let executable = second.join("whisper-cli");
        std::fs::write(&executable, "#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(
            find_in_dirs("whisper-cli", [first, second]),
            Some(executable)
        );
    }

    #[cfg(unix)]
    #[test]
    fn failed_process_cannot_supply_a_stale_transcript() {
        let scratch = tempfile::tempdir().unwrap();
        let path = scratch.path().join("transcript.txt");
        std::fs::write(&path, "stale words").unwrap();
        let failed = Command::new("sh").args(["-c", "exit 1"]).output().unwrap();
        assert!(read_transcript(&failed, &path).is_err());
        let success = Command::new("sh").args(["-c", "exit 0"]).output().unwrap();
        std::fs::write(&path, "  ").unwrap();
        assert_eq!(read_transcript(&success, &path).unwrap(), "");
    }

    #[test]
    #[ignore = "requires installed STT engine, model and OVERSEER_STT_TEST_WAV fixture"]
    fn installed_engine_transcribes_known_speech() {
        let path = std::env::var("OVERSEER_STT_TEST_WAV").expect("set OVERSEER_STT_TEST_WAV");
        let mut wav = hound::WavReader::open(path).unwrap();
        assert_eq!(wav.spec().sample_rate, 16000);
        assert_eq!(wav.spec().channels, 1);
        assert_eq!(wav.spec().bits_per_sample, 16);
        let samples: Vec<_> = wav
            .samples::<i16>()
            .map(|s| s.unwrap() as f32 / 32768.0)
            .collect();
        let text = super::super::local_stt::transcribe_samples_sync(&samples).unwrap();
        println!("Verified transcript: {text}");
        assert!(
            text.to_lowercase().contains("fellow americans"),
            "unexpected transcript: {text}"
        );
    }
}
