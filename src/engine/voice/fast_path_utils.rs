use std::process::Command;

pub async fn handle_volume_fast_path(lower_cmd: &str) -> Option<String> {
    if lower_cmd.contains("volume up")
        || lower_cmd.contains("increase volume")
        || lower_cmd == "louder"
    {
        #[cfg(target_os = "linux")]
        let _ = Command::new("amixer")
            .args(["sset", "Master", "10%+"])
            .output();

        #[cfg(target_os = "macos")]
        let _ = Command::new("osascript")
            .args([
                "-e",
                "set volume output volume ((output volume of (get volume settings)) + 10)",
            ])
            .output();

        #[cfg(target_os = "windows")]
        let _ = Command::new("powershell")
            .args([
                "-Command",
                "$wsh = New-Object -ComObject Wscript.Shell; $wsh.SendKeys([char]175)",
            ])
            .output();

        return Some("Volume increased.".to_string());
    }

    if lower_cmd.contains("volume down")
        || lower_cmd.contains("decrease volume")
        || lower_cmd == "quieter"
    {
        #[cfg(target_os = "linux")]
        let _ = Command::new("amixer")
            .args(["sset", "Master", "10%-"])
            .output();

        #[cfg(target_os = "macos")]
        let _ = Command::new("osascript")
            .args([
                "-e",
                "set volume output volume ((output volume of (get volume settings)) - 10)",
            ])
            .output();

        #[cfg(target_os = "windows")]
        let _ = Command::new("powershell")
            .args([
                "-Command",
                "$wsh = New-Object -ComObject Wscript.Shell; $wsh.SendKeys([char]174)",
            ])
            .output();

        return Some("Volume decreased.".to_string());
    }

    if lower_cmd == "mute" || lower_cmd == "unmute" || lower_cmd.contains("toggle mute") {
        #[cfg(target_os = "linux")]
        let _ = Command::new("amixer")
            .args(["sset", "Master", "toggle"])
            .output();

        #[cfg(target_os = "macos")]
        let _ = Command::new("osascript")
            .args(["-e", "set volume with output muted"])
            .output();

        #[cfg(target_os = "windows")]
        let _ = Command::new("powershell")
            .args([
                "-Command",
                "$wsh = New-Object -ComObject Wscript.Shell; $wsh.SendKeys([char]173)",
            ])
            .output();

        return Some("Volume muted or toggled.".to_string());
    }

    None
}
