use std::process::Command;

pub fn is_wayland() -> bool {
    let xdg = std::env::var("XDG_SESSION_TYPE")
        .unwrap_or_default()
        .to_lowercase();
    let display = std::env::var("WAYLAND_DISPLAY").unwrap_or_default();
    xdg == "wayland" || !display.is_empty()
}

pub fn capture_wayland_screenshot(path: &str) -> Result<(), String> {
    if Command::new("grim").arg("-h").output().is_ok() {
        let output = Command::new("grim").arg(path).output();
        match output {
            Ok(out) if out.status.success() => return Ok(()),
            Ok(out) => {
                return Err(format!(
                    "grim failed with exit code: {:?}",
                    out.status.code()
                ))
            }
            Err(e) => return Err(format!("failed to execute grim: {}", e)),
        }
    }

    Err("grim utility not found on PATH. Install grim for Wayland screenshots.".to_string())
}

pub fn wayland_type_text(text: &str) -> Result<(), String> {
    if Command::new("wtype").arg("-h").output().is_ok() {
        let output = Command::new("wtype").arg(text).output();
        if let Ok(out) = output {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    if Command::new("ydotool").arg("-h").output().is_ok() {
        let output = Command::new("ydotool").args(["type", text]).output();
        if let Ok(out) = output {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    Err("Neither wtype nor ydotool found or functional. Install wtype or ydotool for Wayland input simulation.".to_string())
}

pub fn wayland_press_key(key: &str) -> Result<(), String> {
    let key_lower = key.to_lowercase();
    let mapped_key = match key_lower.as_str() {
        "enter" | "return" => "Return",
        "tab" => "Tab",
        "space" => "space",
        "backspace" => "BackSpace",
        "escape" | "esc" => "Escape",
        _ => key_lower.as_str(),
    };

    if Command::new("wtype").arg("-h").output().is_ok() {
        let output = Command::new("wtype").args(["-k", mapped_key]).output();
        if let Ok(out) = output {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    if Command::new("ydotool").arg("-h").output().is_ok() {
        // ydotool key syntax: ydotool key keycode:state (state 1 = press, 0 = release)
        // Common keycodes: enter = 28, space = 57, backspace = 14
        let key_lower_2 = key.to_lowercase();
        let code = match key_lower_2.as_str() {
            "enter" | "return" => "28",
            "space" => "57",
            "backspace" => "14",
            "tab" => "15",
            "esc" | "escape" => "1",
            _ => "28", // default to enter
        };

        let output = Command::new("ydotool")
            .args(["key", &format!("{}:1", code), &format!("{}:0", code)])
            .output();
        if let Ok(out) = output {
            if out.status.success() {
                return Ok(());
            }
        }
    }

    Err("Neither wtype nor ydotool functional for key press.".to_string())
}
