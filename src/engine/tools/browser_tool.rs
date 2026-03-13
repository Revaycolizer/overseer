use rig::{
    completion::ToolDefinition,
    tool::Tool,
};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::process::Command;

/// Args for the browser_automation tool (Playwright-backed)
#[derive(Deserialize)]
pub struct BrowserArgs {
    /// Action: navigate, search, click, get_links, get_title, get_url, scroll, type, wait_for_text
    pub action: String,
    /// URL for navigate
    pub url: Option<String>,
    /// Text: search query, link text to click, text to type, or text to wait for
    pub text: Option<String>,
    /// CSS selector for click (alternative to text)
    pub selector: Option<String>,
    /// Submit/press-Enter after typing
    pub submit: Option<bool>,
    /// Scroll direction: "up" or "down" (default: "down")
    pub direction: Option<String>,
    /// Scroll amount in pixels (default: 500)
    pub amount: Option<i32>,
    /// For get_links: if true, automatically navigate to the first matching link
    pub navigate: Option<bool>,
    /// X coordinate for OCR click
    pub x: Option<f64>,
    /// Y coordinate for OCR click
    pub y: Option<f64>,
}

pub struct BrowserTool {
    script_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BrowserToolError(String);

impl std::fmt::Display for BrowserToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for BrowserToolError {}

impl BrowserTool {
    pub fn new(script_path: Option<String>) -> Self {
        Self {
            script_path: script_path.unwrap_or_else(|| "src/engine/scripts/web/browser_automation.py".to_string()),
        }
    }
}

impl Tool for BrowserTool {
    const NAME: &'static str = "browser_automation";
    type Args = BrowserArgs;
    type Output = String;
    type Error = BrowserToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Reliable Playwright-based browser automation. Use this for ALL tasks \
                involving Chrome, Firefox, or any web browser. Operates on the real DOM — \
                no OCR, no guessing. Much more reliable than ui_automation for browser tasks.
                
Actions:
- 'navigate': url='https://...' — Navigate to URL completely
- 'search': text='query' — Search DuckDuckGo for query (navigates to search results)
- 'click': text='Link Text' — Click a link/button by its visible text
- 'get_links': text='filter' — Get all visible links (optional text filter)
- 'get_title' — Get the current page title and URL
- 'get_url' — Get the current URL
- 'scroll': direction='down'/'up', amount=500 — Scroll the page
- 'type': text='hello', submit=true — Type into the focused field
- 'wait_for_text': text='Expected Text' — Wait until text appears on page".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Browser action to perform",
                        "enum": ["navigate", "search", "click", "get_links", "get_title", "get_url", "scroll", "type", "wait_for_text", "screenshot"]
                    },
                    "url": {
                        "type": "string",
                        "description": "URL for navigate action (e.g., 'https://github.com')"
                    },
                    "text": {
                        "type": "string",
                        "description": "Search query, link text to click, text to type, or text to wait for"
                    },
                    "selector": {
                        "type": "string",
                        "description": "CSS selector to click (alternative to text)"
                    },
                    "submit": {
                        "type": "boolean",
                        "description": "If true, press Enter after typing"
                    },
                    "direction": {
                        "type": "string",
                        "description": "Scroll direction: 'up' or 'down'",
                        "enum": ["up", "down"]
                    },
                    "amount": {
                        "type": "integer",
                        "description": "Scroll distance in pixels (default: 500)"
                    },
                    "navigate": {
                        "type": "boolean",
                        "description": "For get_links: if true, automatically navigate to first matching link (deterministic/atomic)"
                    },
                    "x": {
                        "type": "number",
                        "description": "X coordinate for click (from OCR/RAW SCREEN TEXT)"
                    },
                    "y": {
                        "type": "number",
                        "description": "Y coordinate for click (from OCR/RAW SCREEN TEXT)"
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
        println!("🌐 BrowserTool Executing: action={}", args.action);

        if let Some(url) = args.url {
            cmd.arg("--url").arg(url);
        }
        if let Some(text) = args.text {
            cmd.arg("--text").arg(text);
        }
        if let Some(selector) = args.selector {
            cmd.arg("--selector").arg(selector);
        }
        if let Some(s) = args.submit {
            if s { cmd.arg("--submit"); }
        }
        if let Some(dir) = args.direction {
            cmd.arg("--direction").arg(dir);
        }
        if let Some(amount) = args.amount {
            cmd.arg("--amount").arg(amount.to_string());
        }
        if let Some(n) = args.navigate {
            if n { cmd.arg("--navigate"); }
        }
        if let Some(x_val) = args.x {
            cmd.arg("--x").arg(x_val.to_string());
        }
        if let Some(y_val) = args.y {
            cmd.arg("--y").arg(y_val.to_string());
        }

        let output = cmd.output()
            .map_err(|e| BrowserToolError(format!("Failed to execute browser script: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        // Print stderr (Playwright debug info) to console
        if !stderr.is_empty() {
            eprintln!("[browser] {}", stderr.trim());
        }

        if output.status.success() {
            Ok(format!("Browser automation completed:\n{}", stdout))
        } else {
            Err(BrowserToolError(format!(
                "Browser script failed:\nSTDOUT: {}\nSTDERR: {}",
                stdout, stderr
            )))
        }
    }
}
