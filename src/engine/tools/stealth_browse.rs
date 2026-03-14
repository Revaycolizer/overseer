use rig::{
    completion::ToolDefinition,
    tool::Tool,
};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Deserialize)]
pub struct StealthBrowseArgs {
    pub url: String,
}

pub struct StealthBrowseTool {
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

impl StealthBrowseTool {
    pub fn new(script_path: Option<String>) -> Self {
        Self {
            script_path: script_path.unwrap_or_else(|| "MISSING_TOOL_PATH".to_string()),
        }
    }
}

impl Tool for StealthBrowseTool {
    const NAME: &'static str = "stealth_browse";
    type Args = StealthBrowseArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Deep text-based research for background agents. \
                Fetches raw visible text from a URL without opening a browser. \
                Use this to verify facts, read pages, and gather specific data (like counts) \
                when a search snippet is not enough.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "url": {
                        "type": "string",
                        "description": "The URL to fetch text from"
                    }
                },
                "required": ["url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let output = Command::new("python3")
            .arg(&self.script_path)
            .arg(&args.url)
            .output()
            .map_err(|e| ToolError(format!("Failed to execute stealth browse: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Err(ToolError(format!("Stealth browse failed: {}\n{}", stdout, stderr)))
        }
    }
}
