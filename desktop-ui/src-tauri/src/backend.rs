use serde::Serialize;
use std::{
    net::TcpListener,
    process::{Child, Command, Stdio},
    sync::Mutex,
};
use tauri::Manager;

#[derive(Clone, Serialize)]
pub struct Connection {
    pub url: String,
    pub token: String,
}
pub struct Backend {
    child: Child,
    pub connection: Connection,
}
pub struct BackendState(pub Mutex<Option<Backend>>);
impl Drop for Backend {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_some() {
            return;
        }
        // Closing this private pipe requests graceful shutdown and also works
        // when the desktop process exits unexpectedly.
        drop(self.child.stdin.take());
        for _ in 0..30 {
            if self.child.try_wait().ok().flatten().is_some() {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
#[tauri::command]
pub fn desktop_connection(
    app: tauri::AppHandle,
    state: tauri::State<BackendState>,
) -> Result<Connection, String> {
    let mut guard = state.0.lock().map_err(|_| "Backend state unavailable")?;
    if let Some(backend) = guard.as_mut() {
        if backend
            .child
            .try_wait()
            .map_err(|e| e.to_string())?
            .is_none()
        {
            return Ok(backend.connection.clone());
        }
        *guard = None;
    }
    let name = if cfg!(windows) {
        "overseer.exe"
    } else {
        "overseer"
    };
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("backend")
        .join(name);
    let executable = if cfg!(debug_assertions) && !bundled.exists() {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/release")
            .join(name)
    } else {
        bundled
    };
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let mut bytes = [0u8; 32];
    getrandom::getrandom(&mut bytes).map_err(|e| e.to_string())?;
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let logs = app.path().app_log_dir().map_err(|e| e.to_string())?;
    std::fs::create_dir_all(&logs).map_err(|e| e.to_string())?;
    let log = logs.join("backend.log");
    if log
        .metadata()
        .map(|m| m.len() > 10_000_000)
        .unwrap_or(false)
    {
        let _ = std::fs::rename(&log, logs.join("backend.previous.log"));
    }
    let file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(log)
        .map_err(|e| e.to_string())?;
    drop(listener);
    let mut command = Command::new(&executable);
    command
        .args(["--server", "--port", &port.to_string()])
        .env("OVERSEER_API_TOKEN", &token)
        .env("OVERSEER_DESKTOP_MANAGED", "1")
        .stdin(Stdio::piped())
        .stderr(file.try_clone().map_err(|e| e.to_string())?)
        .stdout(file);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // no console window
    }
    let child = command
        .spawn()
        .map_err(|e| format!("Could not start bundled backend: {e}"))?;
    let connection = Connection {
        url: format!("ws://127.0.0.1:{port}/api/ws/chat"),
        token,
    };
    *guard = Some(Backend {
        child,
        connection: connection.clone(),
    });
    Ok(connection)
}
