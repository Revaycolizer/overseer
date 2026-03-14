use rig::{
    completion::ToolDefinition,
    tool::Tool,
};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Deserialize)]
pub struct StealthSearchArgs {
    pub query: String,
}

pub struct StealthSearchTool {
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

impl StealthSearchTool {
    pub fn new(script_path: Option<String>) -> Self {
        Self {
            script_path: script_path.unwrap_or_else(|| "MISSING_TOOL_PATH".to_string()),
        }
    }
}

impl Tool for StealthSearchTool {
    const NAME: &'static str = "stealth_search";
    type Args = StealthSearchArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Perform a silent background search to gather information anonymously without opening a browser window. \
                Use this for knowledge queries, finding facts, and general research where visual interaction or site navigation is NOT required.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let output = Command::new("python3")
            .arg(&self.script_path)
            .arg(&args.query)
            .output()
            .map_err(|e| ToolError(format!("Failed to execute stealth search: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            Ok(stdout)
        } else {
            Err(ToolError(format!("Stealth search failed: {}\n{}", stdout, stderr)))
        }
    }
}
