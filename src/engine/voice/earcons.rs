use std::process::Command;

#[cfg(target_os = "linux")]
fn play_linux_bytes(bytes: Vec<u8>) {
    if let Ok(mut child) = Command::new("aplay")
        .args(["-t", "raw", "-r", "8000", "-f", "U8"])
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(&bytes);
        }
    }
}

pub fn play_listening_chime() {
    std::thread::spawn(|| {
        #[cfg(target_os = "macos")]
        { let _ = Command::new("afplay").arg("/System/Library/Sounds/Tink.aiff").output(); }
        #[cfg(target_os = "windows")]
        { let _ = Command::new("powershell").args(["-Command", "[Console]::Beep(1200, 150); [Console]::Beep(1500, 200)"]).output(); }
        #[cfg(target_os = "linux")]
        {
            let mut bytes = Vec::new();
            for i in 0..1200 { bytes.push((128.0 + 127.0 * (i as f32 * 1200.0 * 2.0 * std::f32::consts::PI / 8000.0).sin()) as u8); }
            for i in 0..1600 { bytes.push((128.0 + 127.0 * (i as f32 * 1500.0 * 2.0 * std::f32::consts::PI / 8000.0).sin()) as u8); }
            play_linux_bytes(bytes);
        }
    });
}

pub fn play_thinking_hum() {
    std::thread::spawn(|| {
        #[cfg(target_os = "macos")]
        { let _ = Command::new("afplay").arg("/System/Library/Sounds/Subtle.aiff").output(); }
        #[cfg(target_os = "windows")]
        { let _ = Command::new("powershell").args(["-Command", "[Console]::Beep(180, 500)"]).output(); }
        #[cfg(target_os = "linux")]
        {
            let mut bytes = Vec::new();
            for i in 0..4000 { bytes.push((128.0 + 127.0 * (i as f32 * 180.0 * 2.0 * std::f32::consts::PI / 8000.0).sin()) as u8); }
            play_linux_bytes(bytes);
        }
    });
}

pub fn play_blocked_beep() {
    std::thread::spawn(|| {
        #[cfg(target_os = "macos")]
        { let _ = Command::new("afplay").arg("/System/Library/Sounds/Basso.aiff").output(); }
        #[cfg(target_os = "windows")]
        { let _ = Command::new("powershell").args(["-Command", "[Console]::Beep(400, 250); [Console]::Beep(400, 250)"]).output(); }
        #[cfg(target_os = "linux")]
        {
            let mut bytes = Vec::new();
            for _ in 0..2 {
                for i in 0..2000 { bytes.push((128.0 + 127.0 * (i as f32 * 400.0 * 2.0 * std::f32::consts::PI / 8000.0).sin()) as u8); }
                bytes.extend(std::iter::repeat(128).take(800));
            }
            play_linux_bytes(bytes);
        }
    });
}
