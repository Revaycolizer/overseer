use std::process::Command;
#[cfg(windows)]
use std::os::windows::process::CommandExt;

pub(crate) fn locate_protoc() -> Option<String> {
    let mut candidates = Vec::new();

    if let Ok(explicit) = std::env::var("PROTOC") {
        candidates.push(explicit);
    }

    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(
            current_dir
                .join("tools")
                .join("bin")
                .join(if cfg!(windows) {
                    "protoc.exe"
                } else {
                    "protoc"
                })
                .display()
                .to_string(),
        );
    }

    if cfg!(windows) {
        candidates.push(r"C:\tools\bin\protoc.exe".to_string());
        candidates.push(r"C:\ProgramData\chocolatey\bin\protoc.exe".to_string());
    } else {
        candidates.push("/usr/local/bin/protoc".to_string());
        candidates.push("/usr/bin/protoc".to_string());
    }

    for candidate in candidates {
        if !candidate.trim().is_empty() && std::path::Path::new(candidate.trim()).exists() {
            return Some(candidate);
        }
    }

    #[cfg(windows)]
    const CREATE_NO_WINDOW: u32 = 0x08000000;

    let mut probe = if cfg!(windows) {
        Command::new("where")
    } else {
        Command::new("which")
    };
    probe.arg("protoc");
    #[cfg(windows)]
    probe.creation_flags(CREATE_NO_WINDOW);

    if let Ok(output) = probe.output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_string);
            if path.is_some() {
                return path;
            }
        }
    }

    None
}
