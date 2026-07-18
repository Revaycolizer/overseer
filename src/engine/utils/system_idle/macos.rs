#[cfg(target_os = "macos")]
pub fn get_idle_secs() -> u64 {
    use std::process::Command;

    let output = Command::new("ioreg")
        .args(["-c", "IOHIDSystem", "-d", "4"])
        .output()
        .unwrap_or_default();

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        if line.contains("HIDIdleTime") {
            if let Some(val) = line.split('=').nth(1) {
                if let Ok(ns) = val.trim().parse::<u64>() {
                    // HIDIdleTime is in nanoseconds
                    return ns / 1_000_000_000;
                }
            }
        }
    }
    0
}
