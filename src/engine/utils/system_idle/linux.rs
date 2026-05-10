#[cfg(target_os = "linux")]
pub fn get_idle_secs() -> u64 {
    // Primary: X11 Screen Saver extension (X11 + XWayland)
    if let Some(secs) = get_x11_idle_secs() {
        return secs;
    }

    // Fallback: GNOME Mutter D-Bus (pure Wayland / GNOME)
    if let Some(secs) = get_gnome_idle_secs() {
        return secs;
    }

    // Last resort: KDE KWin D-Bus
    if let Some(secs) = get_kde_idle_secs() {
        return secs;
    }

    0
}

#[cfg(target_os = "linux")]
fn get_x11_idle_secs() -> Option<u64> {
    use x11rb::connection::Connection;
    use x11rb::protocol::screensaver;

    let (conn, screen_num) = x11rb::connect(None).ok()?;
    let screen = &conn.setup().roots[screen_num];
    let reply = screensaver::query_info(&conn, screen.root)
        .ok()?
        .reply()
        .ok()?;

    // ms_since_user_input is in milliseconds
    Some((reply.ms_since_user_input / 1000) as u64)
}

#[cfg(target_os = "linux")]
fn get_gnome_idle_secs() -> Option<u64> {
    use std::process::Command;

    let output = Command::new("gdbus")
        .args([
            "call",
            "--session",
            "--dest", "org.gnome.Mutter.IdleMonitor",
            "--object-path", "/org/gnome/Mutter/IdleMonitor/Core",
            "--method", "org.gnome.Mutter.IdleMonitor.GetIdletime",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let ms: u64 = stdout
        .trim()
        .trim_matches(|c| c == '(' || c == ')' || c == ',')
        .split_whitespace()
        .last()?
        .parse()
        .ok()?;

    Some(ms / 1000)
}

#[cfg(target_os = "linux")]
fn get_kde_idle_secs() -> Option<u64> {
    use std::process::Command;

    let output = Command::new("qdbus")
        .args([
            "org.kde.KWin",
            "/Scripting",
            "org.kde.kwin.Scripting.idleTime",
        ])
        .output()
        .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let ms: u64 = stdout.trim().parse().ok()?;
    Some(ms / 1000)
}
