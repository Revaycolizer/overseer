use crate::engine::tools::error::DynamicToolError;

#[cfg(windows)]
pub async fn handle_launch(app: &str) -> Result<String, DynamicToolError> {
    use crate::engine::tools::os_tool::powershell::run_powershell_hidden;
    use crate::engine::tools::os_tool::fields::escape_powershell_single_quoted;

    let escaped = escape_powershell_single_quoted(app);
    let script = format!(
        "Start-Process '{app}'; [pscustomobject]@{{ action='launch'; app='{app}'; ok=$true }} | ConvertTo-Json -Compress",
        app = escaped
    );
    run_powershell_hidden(&script, "os_tool").await
}

/// Resolve a target that should be opened as a folder/file (returns a real path)
/// rather than run as a program: common folder names, `~/…`, or an existing path.
#[cfg(not(windows))]
fn resolve_path(target: &str) -> Option<String> {
    let home = dirs::home_dir()?;
    let mut t = target.trim().to_lowercase();
    if let Some(stripped) = t.strip_suffix(" folder") {
        t = stripped.trim().to_string();
    }
    let dir = match t.as_str() {
        "desktop" => home.join("Desktop"),
        "downloads" | "download" => home.join("Downloads"),
        "documents" | "docs" => home.join("Documents"),
        "pictures" | "photos" => home.join("Pictures"),
        "music" => home.join("Music"),
        "videos" | "movies" => home.join("Videos"),
        "home" => home.clone(),
        _ => {
            let p = if let Some(rest) = target.strip_prefix("~/") {
                home.join(rest)
            } else {
                std::path::PathBuf::from(target)
            };
            if (target.starts_with('/') || target.starts_with("~/")) && p.exists() {
                return Some(p.to_string_lossy().to_string());
            }
            return None;
        }
    };
    Some(dir.to_string_lossy().to_string())
}

/// Normalize a URL-ish target into something `xdg-open` can handle.
#[cfg(not(windows))]
fn as_url(target: &str) -> Option<String> {
    let t = target.trim();
    if t.contains("://") {
        return Some(t.to_string());
    }
    if t.starts_with("www.") {
        return Some(format!("https://{t}"));
    }
    // bare domain like example.com — no spaces, no slash, has a dot
    if !t.contains(' ') && !t.contains('/') && t.contains('.') && t.split('.').all(|p| !p.is_empty()) {
        return Some(format!("https://{t}"));
    }
    None
}

#[cfg(not(windows))]
pub async fn handle_launch(app: &str) -> Result<String, DynamicToolError> {
    use serde_json::json;
    use tokio::process::Command;
    use crate::engine::tools::os_tool::handlers::focus_app::desktop_file::find_linux_desktop_file;

    let ok = |method: &str, detail: String| {
        Ok(json!({
            "action": "launch",
            "app": app,
            "success": true,
            "method": method,
            "summary": detail,
        })
        .to_string())
    };

    if cfg!(target_os = "macos") {
        let status = Command::new("open").arg("-a").arg(app).status().await.map_err(|e| {
            DynamicToolError::ExecutionFailed { tool: "os_tool".to_string(), details: format!("open -a {app}: {e}") }
        })?;
        if !status.success() {
            return Err(DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("'open -a' could not launch '{app}'."),
            });
        }
        return ok("open", format!("Opened '{app}'."));
    }

    // --- Linux ---
    // 1. Folders / files / known locations → file manager (xdg-open a real path).
    if let Some(path) = resolve_path(app) {
        return match Command::new("xdg-open").arg(&path).spawn() {
            Ok(_) => ok("xdg-open", format!("Opened {path}.")),
            Err(e) => Err(DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("Couldn't open {path}: {e}"),
            }),
        };
    }

    // 2. URLs → browser.
    if let Some(url) = as_url(app) {
        return match Command::new("xdg-open").arg(&url).spawn() {
            Ok(_) => ok("xdg-open", format!("Opened {url}.")),
            Err(e) => Err(DynamicToolError::ExecutionFailed {
                tool: "os_tool".to_string(),
                details: format!("Couldn't open {url}: {e}"),
            }),
        };
    }

    // 3. Application by name — try the binary, then a matching .desktop entry.
    let bin = app.split_whitespace().next().unwrap_or(app);
    if Command::new(bin).spawn().is_ok() {
        return ok("exec", format!("Launched '{app}'."));
    }
    if let Some(desktop_file) = find_linux_desktop_file(app).await {
        if Command::new("gtk-launch").arg(&desktop_file).spawn().is_ok() {
            return ok("desktop-file", format!("Launched '{app}' via {desktop_file}."));
        }
    }

    // Nothing worked — report honestly instead of a false success.
    Err(DynamicToolError::ExecutionFailed {
        tool: "os_tool".to_string(),
        details: format!(
            "Couldn't find an app called '{app}'. Try its exact command name (e.g. 'cheese' for the camera, 'nautilus' for files), a path like ~/Desktop, or a URL."
        ),
    })
}
