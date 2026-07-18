pub fn detect_install_command() -> Option<&'static str> {
    if which::which("apt").is_ok() {
        Some("sudo apt install ffmpeg")
    } else if which::which("pacman").is_ok() {
        Some("sudo pacman -S ffmpeg")
    } else if which::which("dnf").is_ok() {
        Some("sudo dnf install ffmpeg")
    } else if which::which("brew").is_ok() {
        Some("brew install ffmpeg")
    } else {
        None
    }
}
