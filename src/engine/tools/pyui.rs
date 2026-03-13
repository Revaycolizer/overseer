use rig::{
    completion::ToolDefinition,
    tool::Tool,
};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Deserialize)]
pub struct PyUIArgs {
    pub action: String,
    pub app: Option<String>,
    pub text: Option<String>,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub key: Option<String>,
    pub no_launch: Option<bool>,
    pub clear: Option<bool>,
    pub submit: Option<bool>,
}

pub struct PyUITool {
    script_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolError(String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolError {}

impl PyUITool {
    pub fn new(script_path: Option<String>) -> Self {
        Self {
            script_path: script_path.unwrap_or_else(|| "src/engine/scripts/desktop/ui_automation.py".to_string()),
        }
    }
}

impl Tool for PyUITool {
    const NAME: &'static str = "ui_automation";
    type Args = PyUIArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Reliable UI automation using Python. 
            Best used with Vision Tool for adaptive coordinates.
            actions: 
            - 'launch': app='name' (e.g., 'firefox', 'chromium')
            - 'click': x=100, y=200, app='name' (Click coordinates, auto-focuses app) OR text='Image', app='name'
            - 'type': text='hello', key='enter', app='name' (Types text, optionally focus app)
            - 'search': app='name', text='query' (HYBRID: Automatically finds search fields via UI Tree, OCR, and Heuristics. Use this for ANY search task in ANY app.)
            - 'scroll': text='down'/'up', y=300 (Explore current page without resetting context)
            - 'focus': app='name' (Focus an existing window)
            - 'play_song': app='neontune', text='song name' (Specific for music apps)
            - 'get_screen_text': app='name', text='optional_filter' (Returns JSON list of text. Use for in-page navigation!)".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Action to perform: launch, click, type, search, scroll, focus, play_song, get_screen_text, wait, wait_for_text, get_ui_tree, wait_for_settle",
                        "enum": ["launch", "click", "type", "search", "scroll", "focus", "play_song", "get_screen_text", "wait", "wait_for_text", "get_ui_tree", "wait_for_settle"]
                    },
                    "app": {
                        "type": "string",
                        "description": "Application name to launch or focus (e.g., 'chrome', 'terminal', 'slack')"
                    },
                    "text": {
                        "type": "string",
                        "description": "Text to type or search query"
                    },
                    "x": {
                        "type": "integer",
                        "description": "X coordinate (for click action)"
                    },
                    "y": {
                        "type": "integer",
                        "description": "Y coordinate (for click action)"
                    },
                    "key": {
                        "type": "string",
                        "description": "Key to press after typing (for type action)"
                    },
                    "no_launch": {
                        "type": "boolean",
                        "description": "If true, will only attempt to focus and search without launching a new instance."
                    },
                    "clear": {
                        "type": "boolean",
                        "description": "If true, clears the input field before typing (Ctrl+A, Backspace)."
                    },
                    "submit": {
                        "type": "boolean",
                        "description": "If true, presses 'Enter' after typing."
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let mut cmd = Command::new("python3");
        cmd.arg(&self.script_path);
        cmd.arg("--action").arg(&args.action);
        println!("🛠️ PyUITool Executing: {:?}", cmd);

        if let Some(app) = args.app {
            cmd.arg("--app").arg(app);
        }
        if let Some(text) = args.text {
            cmd.arg("--text").arg(text);
        }
        if let Some(x) = args.x {
            cmd.arg("--x").arg(x.to_string());
        }
        if let Some(y) = args.y {
            cmd.arg("--y").arg(y.to_string());
        }
        if let Some(key) = args.key {
            cmd.arg("--key").arg(key);
        }
        if let Some(nl) = args.no_launch {
            if nl { cmd.arg("--no-launch"); }
        }
        if let Some(c) = args.clear {
            if c { cmd.arg("--clear"); }
        }
        if let Some(s) = args.submit {
            if s { cmd.arg("--submit"); }
        }

        let output = cmd.output()
            .map_err(|e| ToolError(format!("Failed to execute Python script: {}", e)))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            Ok(format!("UI automation completed:\n{}", stdout))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(ToolError(format!("Python script failed: {}", stderr)))
        }
    }
}
