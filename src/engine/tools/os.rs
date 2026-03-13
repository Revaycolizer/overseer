use rig::{
    completion::{ToolDefinition, PromptError},
    tool::Tool,
};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::Command;
use std::fs;
use std::io::{self, Write};
use std::result;

#[derive(Deserialize)]
pub struct OSArgs {
    pub action: String, // ListFiles, SearchFiles, ReadFile, WriteFile, DeleteFile, ExecuteCommand, OpenFile, RunApp, FocusApp, CheckApp, InstallApp, IsAppRunning
    pub path: Option<String>,
    pub pattern: Option<String>,
    pub root: Option<String>,
    pub content: Option<String>,
    pub command: Option<String>,
    pub args: Option<Vec<String>>,
    pub app_name: Option<String>,
    pub name: Option<String>,
}

pub struct OSTool {}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolError(String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolError {}

impl From<PromptError> for ToolError {
    fn from(e: PromptError) -> Self {
        ToolError(e.to_string())
    }
}

impl OSTool {
    pub fn new() -> Self {
        Self {}
    }

    fn expand_home(path: &str) -> String {
        if path.starts_with("~/") {
            let home = if cfg!(target_os = "windows") {
                std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
            } else {
                std::env::var("HOME")
            }.unwrap_or_else(|_| ".".to_string());
            path.replacen("~", &home, 1)
        } else {
            path.to_string()
        }
    }

    fn list_files(path: &str) -> Result<String, ToolError> {
        let expanded = Self::expand_home(path);
        let entries = fs::read_dir(&expanded)
            .map_err(|e| ToolError(format!("Failed to read directory {}: {}", expanded, e)))?;

        let mut result = Vec::new();
        for entry in entries {
            if let Ok(entry) = entry {
                let file_name = entry.file_name().to_string_lossy().to_string();
                let file_type = if entry.path().is_dir() { "DIR" } else { "FILE" };
                result.push(format!("[{}] {}", file_type, file_name));
            }
        }
        Ok(result.join("\n"))
    }

    fn search_files(pattern: &str, root: Option<String>) -> Result<String, ToolError> {
        let root_path = root.unwrap_or_else(|| "~".to_string());
        let expanded_root = Self::expand_home(&root_path);
        let mut matches = Vec::new();

        println!("🔍 Searching for '{}' starting from {}...", pattern, expanded_root);
        io::stdout().flush().ok();
        
        Self::visit_dirs(Path::new(&expanded_root), pattern, &mut matches, 0, 5)?;
        
        if matches.is_empty() {
            println!("❌ No matching files found.");
            Ok("No matching files found.".to_string())
        } else {
            println!("✅ Found {} matches.", matches.len());
            Ok(matches.join("\n"))
        }
    }

    fn visit_dirs(dir: &Path, pattern: &str, matches: &mut Vec<String>, depth: u32, max_depth: u32) -> Result<(), ToolError> {
        if depth > max_depth {
            return Ok(());
        }

        if dir.is_dir() {
            let file_name = dir.file_name().unwrap_or_default().to_string_lossy();
            
            // Skip common huge/hidden directories
            if file_name.starts_with('.') && file_name != "." && file_name != ".." {
                return Ok(());
            }
            if ["node_modules", "target", "vendor", "Library", "cache", "Cache", ".cache", "tmp"].contains(&file_name.as_ref()) {
                return Ok(());
            }

            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_dir() {
                            let _ = Self::visit_dirs(&path, pattern, matches, depth + 1, max_depth);
                        } else {
                            if let Some(filename) = path.file_name() {
                                if filename.to_string_lossy().to_lowercase().contains(&pattern.to_lowercase()) {
                                    matches.push(path.to_string_lossy().to_string());
                                }
                            }
                        }
                    }
                    if matches.len() > 100 { break; }
                }
            }
        }
        Ok(())
    }

    fn open_file(path: &str) -> Result<String, ToolError> {
        let expanded = Self::expand_home(path);
        println!("🚀 Opening: {}", expanded);
        io::stdout().flush().ok();

        if cfg!(target_os = "macos") {
            Command::new("open")
                .arg(&expanded)
                .spawn()
                .map_err(|e| ToolError(format!("Failed to open file {}: {}", expanded, e)))?;
        } else if cfg!(target_os = "windows") {
            Command::new("cmd")
                .args(["/c", "start", "", &expanded])
                .spawn()
                .map_err(|e| ToolError(format!("Failed to open file {}: {}", expanded, e)))?;
        } else {
            // Linux handling
            if expanded.ends_with(".desktop") {
                let filename = Path::new(&expanded).file_name().unwrap_or_default().to_string_lossy();
                // Try gtk-launch first
                if Command::new("gtk-launch").arg(&*filename).spawn().is_ok() {
                    println!("✨ Launched via gtk-launch");
                    return Ok(format!("Launched app via gtk-launch: {}", filename));
                }
                
                // Fallback: parse Exec line
                if let Ok(content) = fs::read_to_string(&expanded) {
                    for line in content.lines() {
                        if line.starts_with("Exec=") {
                            let exec_val = line.trim_start_matches("Exec=");
                            // Extract just the first command part, removing placeholders
                            let parts: Vec<&str> = exec_val.split_whitespace().collect();
                            if let Some(cmd) = parts.first() {
                                let binary = cmd.trim();
                                if Command::new(binary).spawn().is_ok() {
                                    println!("✨ Launched via Exec binary: {}", binary);
                                    return Ok(format!("Launched app via binary: {}", binary));
                                }
                            }
                        }
                    }
                }
            }

            // Default to xdg-open
            Command::new("xdg-open")
                .arg(&expanded)
                .spawn()
                .map_err(|e| ToolError(format!("Failed to open file {}: {}", expanded, e)))?;
        }

        Ok(format!("Opened {}", expanded))
    }

    fn run_app_by_name(name: &str, args: Option<Vec<String>>) -> Result<String, ToolError> {
        let cmd_args = args.clone().unwrap_or_default();
        println!("🚀 Attempting to launch app: '{}' with args: {:?}", name, cmd_args);
        
        // 1. Try to find the binary path first (handling case-sensitivity)
        let find_cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
        
        let path = 'search: loop {
            // 1. Try exact name
            if let Ok(out) = Command::new(find_cmd).arg(name).output() {
                if out.status.success() {
                    break 'search Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
                }
            }

            // 2. Try lowercase name
            let low_name = name.to_lowercase();
            if low_name != name {
                if let Ok(out) = Command::new(find_cmd).arg(&low_name).output() {
                    if out.status.success() {
                        break 'search Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
                    }
                }
            }

            // 3. Try aliases
            let aliases = match low_name.as_str() {
                "chrome" => vec!["google-chrome", "google-chrome-stable", "chromium"],
                "firefox" => vec!["firefox-esr", "firefox-bin"],
                "code" => vec!["visual-studio-code", "code-oss"],
                "spotify" => vec!["spotify"],
                "neontune" => vec!["neontune"],
                "calculator" => vec!["gnome-calculator", "kcalc", "gcalctool"],
                _ => vec![],
            };

            for alias in aliases {
                if let Ok(out) = Command::new(find_cmd).arg(alias).output() {
                    if out.status.success() {
                        break 'search Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
                    }
                }
            }
            
            break None;
        };
        let mut launched = false;
        let mut result_msg = String::new();
        let mut pid: Option<u32> = None;

        if let Some(binary_path) = path {
            println!("✨ Found binary at: {}. Launching...", binary_path);
            if let Ok(child) = Command::new(&binary_path).args(&cmd_args).spawn() {
                launched = true;
                pid = Some(child.id());
                result_msg = format!("Successfully launched '{}' (PID: {:?}) from path: {} with args: {:?}", name, pid, binary_path, cmd_args);
            }
        }

        // 3. Fallback: Search for desktop files or other matches
        if !launched {
            println!("🔎 Binary not found in PATH or failed to launch. Searching for file matches for '{}'...", name);
            let search_results = Self::search_files(name, None)?;
            if !search_results.contains("No matching files found") {
                let first_match = search_results.lines().next().unwrap();
                println!("📂 Found matching file: {}. Attempting to open...", first_match);
                // Note: opening a file might not give us the PID of the app easily if it uses xdg-open/open
                result_msg = Self::open_file(first_match)?;
                launched = true;
            }
        }

        if launched {
            // Wait longer for window to appear and retry focusing
            println!("⏳ Waiting for window to appear...");
            for i in 1..=10 {
                std::thread::sleep(std::time::Duration::from_millis(1000));
                
                // Try focusing by PID if available, else by name
                let focus_result = if let Some(p) = pid {
                    Self::focus_pid(p)
                } else {
                    Self::focus_app(name)
                };

                match focus_result {
                    Ok(_) => {
                        println!("🎯 Focus success on attempt {}", i);
                        break;
                    },
                    Err(_) => {
                        // Fallback to name if PID failed (or wasn't available)
                        if pid.is_some() {
                             if let Ok(_) = Self::focus_app(name) {
                                 println!("🎯 Focus success (fallback to name) on attempt {}", i);
                                 break;
                             }
                        }
                        if i < 10 {
                             println!("... Still waiting for window '{}' (pid {:?}) (attempt {})...", name, pid, i);
                        } else {
                             println!("❌ Failed to focus app after 10 attempts");
                        }
                    }
                }
            }
            Ok(result_msg)
        } else {
            Err(ToolError(format!("Could not find or launch application '{}'.", name)))
        }
    }

    fn focus_pid(pid: u32) -> Result<String, ToolError> {
        println!("🎯 Attempting to focus PID: {}", pid);
         if cfg!(target_os = "linux") {
             // Try xdotool search --pid first
            let search = Command::new("xdotool")
                .args(["search", "--pid", &pid.to_string(), "--limit", "1"])
                .output();
            
            if let Ok(output) = search {
                if output.status.success() {
                    let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !window_id.is_empty() {
                        if Command::new("xdotool").args(["windowactivate", &window_id]).status().map(|s| s.success()).unwrap_or(false) {
                            return Ok(format!("Focused PID {} (window id: {}) via xdotool", pid, window_id));
                        }
                    }
                }
            }
            return Err(ToolError(format!("Could not find window for PID {}", pid)));
         }
         // Fallback for non-linux or if xdotool fails
         Err(ToolError("PID focusing only supported on Linux with xdotool".to_string()))
    }

    fn focus_app(name: &str) -> Result<String, ToolError> {
        println!("🎯 Attempting to focus app: '{}'", name);
        if cfg!(target_os = "macos") {
            Command::new("osascript")
                .args(["-e", &format!("tell application \"{}\" to activate", name)])
                .spawn()
                .map_err(|e| ToolError(format!("Failed to focus app on macOS: {}", e)))?;
        } else if cfg!(target_os = "windows") {
             let ps_cmd = format!(
                "$wshell = New-Object -ComObject WScript.Shell; \
                 $wshell.AppActivate('{}')",
                name
            );
            Command::new("powershell")
                .args(["-Command", &ps_cmd])
                .spawn()
                .map_err(|e| ToolError(format!("Failed to focus app on Windows: {}", e)))?;
        } else {
            // Linux: Try multiple strategies
            
            // 1. Try wmctrl by exact name
            if Command::new("wmctrl").args(["-a", name]).status().map(|s| s.success()).unwrap_or(false) {
                 return Ok(format!("Focused '{}' via wmctrl -a", name));
            }

            // 2. Try xdotool windowactivate by name (case-insensitive search)
            let search = Command::new("xdotool")
                .args(["search", "--name", "--limit", "1", name])
                .output();
            
            if let Ok(output) = search {
                if output.status.success() {
                    let window_id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !window_id.is_empty() {
                        if Command::new("xdotool").args(["windowactivate", &window_id]).status().map(|s| s.success()).unwrap_or(false) {
                            return Ok(format!("Focused '{}' (window id: {}) via xdotool", name, window_id));
                        }
                    }
                }
            }

            // 3. Try lowercase name
            let low_name = name.to_lowercase();
            if low_name != name {
                 let _ = Command::new("wmctrl").args(["-a", &low_name]).status();
            }

            return Err(ToolError(format!("Could not find a window matching '{}'", name)));
        }
        Ok(format!("Focus attempt sent for '{}'", name))
    }

    fn check_app(name: &str) -> result::Result<String, ToolError> {
        let find_cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
        
        // 1. Try exact name
        if let Ok(output) = Command::new(find_cmd).arg(name).output() {
            if output.status.success() {
                 let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                 return Ok(format!("App '{}' found at: {}", name, path));
            }
        }

        // 2. Try lowercase name
        let low_name = name.to_lowercase();
        if low_name != name {
             if let Ok(output) = Command::new(find_cmd).arg(&low_name).output() {
                if output.status.success() {
                     let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                     return Ok(format!("App '{}' (found as '{}') at: {}", name, low_name, path));
                }
            }
        }

        // 3. Try common aliases
        let aliases = match low_name.as_str() {
            "chrome" => vec!["google-chrome", "google-chrome-stable", "chromium"],
            "firefox" => vec!["firefox-esr", "firefox-bin"],
            "code" => vec!["visual-studio-code", "code-oss"],
            "calculator" => vec!["gnome-calculator", "kcalc", "gcalctool"],
            _ => vec![],
        };

        for alias in aliases {
            if let Ok(output) = Command::new(find_cmd).arg(alias).output() {
                if output.status.success() {
                     let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                     return Ok(format!("App '{}' (found as '{}') at: {}", name, alias, path));
                }
            }
        }
        
        Ok(format!("App '{}' not found.", name))
    }

    fn install_app(name: &str) -> result::Result<String, ToolError> {
        if cfg!(target_os = "linux") {
            let package_managers = [
                ("apt-get", vec!["install", "-y"]),
                ("snap", vec!["install"]),
                ("pacman", vec!["-S", "--noconfirm"]),
                ("dnf", vec!["install", "-y"]),
            ];

            for (pm, args) in package_managers.iter() {
                if Command::new("which").arg(pm).output().map(|o| o.status.success()).unwrap_or(false) {
                    println!("🚀 Attempting to install '{}' using {}...", name, pm);
                    let mut cmd_args = vec![*pm];
                    cmd_args.extend(args.clone());
                    cmd_args.push(name);
                    
                    let cmd_str = format!("sudo {} {} {}", pm, args.join(" "), name);
                    
                    let status = Command::new("pkexec")
                        .args(&cmd_args)
                        .status();
                        
                    match status {
                        Ok(s) if s.success() => return Ok(format!("Successfully installed '{}'.", name)),
                        Ok(_) => return Err(ToolError(format!("Installation failed. Try running manually: {}", cmd_str))),
                        Err(e) => return Err(ToolError(format!("Failed to execute installer: {}", e))),
                    }
                }
            }
            Err(ToolError("No supported package manager found (apt, snap, pacman, dnf). Please install manually.".to_string()))
        } else {
            Err(ToolError("Automatic installation is only supported on Linux.".to_string()))
        }
    }

    fn is_app_running(name: &str) -> result::Result<String, ToolError> {
        let (cmd, args) = if cfg!(target_os = "windows") {
            ("tasklist", vec!["/FI".to_string(), format!("IMAGENAME eq {}", name)])
        } else {
            ("pgrep", vec!["-f".to_string(), "-i".to_string(), name.to_string()])
        };

        match Command::new(cmd).args(&args).output() {
            Ok(output) => {
                if output.status.success() {
                     // On Linux pgrep returns 0 if found. On Windows tasklist always returns 0 but prints "No tasks..." if not found.
                     let stdout = String::from_utf8_lossy(&output.stdout);
                     if cfg!(target_os = "windows") && stdout.contains("No tasks") {
                         Ok(format!("App '{}' is NOT running.", name))
                     } else {
                         Ok(format!("App '{}' is RUNNING.", name))
                     }
                } else {
                    Ok(format!("App '{}' is NOT running.", name))
                }
            },
            Err(e) => Err(ToolError(format!("Failed to check process status: {}", e))),
        }
    }
}

impl Tool for OSTool {
    const NAME: &'static str = "os_control";

    type Args = OSArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Control the OS. Actions: RunApp, SearchFiles, OpenFile, ReadFile, WriteFile, DeleteFile, ExecuteCommand.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": { 
                        "type": "string", 
                        "enum": ["ListFiles", "SearchFiles", "ReadFile", "WriteFile", "DeleteFile", "ExecuteCommand", "OpenFile", "RunApp", "FocusApp", "CheckApp", "InstallApp", "IsAppRunning"],
                        "description": "The action to perform." 
                    },
                    "path": { "type": "string", "description": "Path or file/app name." },
                    "name": { "type": "string", "description": "App name for FocusApp/RunApp/CheckApp/InstallApp/IsAppRunning." },
                    "content": { "type": "string", "description": "Content for WriteFile." },
                    "command": { "type": "string", "description": "Command for ExecuteCommand." },
                    "args": { "type": "array", "items": { "type": "string" }, "description": "Arguments for ExecuteCommand." }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        match args.action.as_str() {
            "ListFiles" => {
                let path = args.path.clone().ok_or_else(|| ToolError("Missing path for ListFiles".into()))?;
                println!("📂 Listing files in {}...", path);
                Self::list_files(&path)
            },
            "SearchFiles" => {
                let pattern = args.path.clone().or(args.pattern.clone()).ok_or_else(|| ToolError("Missing pattern for SearchFiles".into()))?;
                Self::search_files(&pattern, args.root.clone())
            },
            "ReadFile" => {
                let path = args.path.clone().ok_or_else(|| ToolError("Missing path for ReadFile".into()))?;
                let expanded = Self::expand_home(&path);
                println!("📖 Reading file {}...", expanded);
                fs::read_to_string(&expanded)
                    .map_err(|e| ToolError(format!("Failed to read file {}: {}", expanded, e)))
            }
            "WriteFile" => {
                let path = args.path.clone().ok_or_else(|| ToolError("Missing path for WriteFile".into()))?;
                let content = args.content.clone().ok_or_else(|| ToolError("Missing content for WriteFile".into()))?;
                let expanded = Self::expand_home(&path);
                println!("📝 Writing to {}...", expanded);
                fs::write(&expanded, content)
                    .map_err(|e| ToolError(format!("Failed to write file {}: {}", expanded, e)))?;
                Ok(format!("Successfully wrote to {}", expanded))
            }
            "DeleteFile" => {
                let path = args.path.clone().ok_or_else(|| ToolError("Missing path for DeleteFile".into()))?;
                let expanded = Self::expand_home(&path);
                println!("🗑️ Deleting {}...", expanded);
                fs::remove_file(&expanded)
                    .map_err(|e| ToolError(format!("Failed to delete file {}: {}", expanded, e)))?;
                Ok(format!("Successfully deleted {}", expanded))
            }
            "ExecuteCommand" => {
                let command = args.command.clone().or(args.path.clone()).ok_or_else(|| ToolError("Missing command for ExecuteCommand".into()))?;
                let cmd_args = args.args.clone().unwrap_or_default();
                println!("💻 Executing: {} {}", command, cmd_args.join(" "));
                let output = Command::new(command)
                    .args(cmd_args)
                    .output()
                    .map_err(|e| ToolError(format!("Failed to execute command: {}", e)))?;
                
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                
                Ok(format!("STDOUT:\n{}\n\nSTDERR:\n{}", stdout, stderr))
            }
            "OpenFile" => {
                let path = args.path.clone().ok_or_else(|| ToolError("Missing path for OpenFile".into()))?;
                Self::open_file(&path)
            },
            "RunApp" => {
                let name = args.path.clone().or(args.app_name.clone()).or(args.name.clone()).ok_or_else(|| ToolError("Missing app name for RunApp".into()))?;
                Self::run_app_by_name(&name, args.args)
            },
            "FocusApp" => {
                let name = args.name.clone().or(args.path.clone()).ok_or_else(|| ToolError("Missing app name for FocusApp".into()))?;
                Self::focus_app(&name)
            },
            "CheckApp" => {
                let name = args.path.clone().or(args.app_name.clone()).or(args.name.clone()).ok_or_else(|| ToolError("Missing app name for CheckApp".into()))?;
                Self::check_app(&name)
            },
            "InstallApp" => {
                let name = args.path.clone().or(args.app_name.clone()).or(args.name.clone()).ok_or_else(|| ToolError("Missing app name for InstallApp".into()))?;
                Self::install_app(&name)
            },
            "IsAppRunning" => {
                 let name = args.path.clone().or(args.app_name.clone()).or(args.name.clone()).ok_or_else(|| ToolError("Missing app name for IsAppRunning".into()))?;
                 Self::is_app_running(&name)
            },
            _ => Err(ToolError(format!("Unknown action: {}", args.action))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expand_home() {
        let path = "~/test.txt";
        let expanded = OSTool::expand_home(path);
        assert!(expanded.contains("test.txt"));
        assert!(!expanded.starts_with("~"));
    }

    #[test]
    fn test_list_files() {
        let result = OSTool::list_files(".");
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("src") || output.contains("Cargo.toml"));
    }

    #[test]
    fn test_search_files() {
        let result = OSTool::search_files("os.rs", Some("src".to_string()));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("os.rs"));
    }

    #[tokio::test]
    async fn test_read_write_delete_file() {
        let path = "test_file.tmp";
        let content = "Hello OSTool";
        
        // Write
        let write_result = OSTool::new().call(OSArgs {
            action: "WriteFile".to_string(),
            path: Some(path.to_string()),
            pattern: None,
            root: None,
            content: Some(content.to_string()),
            command: None,
            args: None,
            app_name: None,
            name: None,
        }).await;
        assert!(write_result.is_ok());

        // Read
        let read_result = OSTool::new().call(OSArgs {
            action: "ReadFile".to_string(),
            path: Some(path.to_string()),
            pattern: None,
            root: None,
            content: None,
            command: None,
            args: None,
            app_name: None,
            name: None,
        }).await;
        assert!(read_result.is_ok());
        assert_eq!(read_result.unwrap(), content);

        // Delete
        let delete_result = OSTool::new().call(OSArgs {
            action: "DeleteFile".to_string(),
            path: Some(path.to_string()),
            pattern: None,
            root: None,
            content: None,
            command: None,
            args: None,
            app_name: None,
            name: None,
        }).await;
        assert!(delete_result.is_ok());
        
        // Verify deleted
        assert!(!Path::new(path).exists());
    }
}
