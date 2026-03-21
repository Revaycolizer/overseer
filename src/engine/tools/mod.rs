use rig::tool::Tool;
use rig::completion::ToolDefinition;
use serde_json::json;
use crate::engine::utils::discovery::{ToolInfo, get_tool_executable_path};
use std::process::Command;
pub mod charting;

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
pub struct DynamicToolError(pub String);

impl std::fmt::Display for DynamicToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for DynamicToolError {}

// Internal tool execution logic — shared by DynamicToolRouter and call_dynamic_tool
async fn call_dynamic_tool_logic(info: &ToolInfo, args: serde_json::Value) -> Result<String, DynamicToolError> {
    match info.language.as_str() {
        "python" => {
            let executable_path = get_tool_executable_path(info)
                .ok_or_else(|| DynamicToolError("No executable path for tool".into()))?;

            let mut cmd = Command::new("python3");
            cmd.arg(&executable_path);

            let input_str = if let Some(input) = args.get("input") {
                if input.is_string() {
                    input.as_str().unwrap_or("").to_string()
                } else {
                    input.to_string()
                }
            } else if let Some(inner) = args.get("args") {
                inner.to_string()
            } else if args.is_object() {
                // If it's a JSON object, try to extract the most likely string value
                let map = args.as_object().unwrap();
                if map.len() == 1 {
                    // Only one key? Take its value (handles {"url": "..."} or {"query": "..."} gracefully)
                    let val = map.values().next().unwrap();
                    if val.is_string() { val.as_str().unwrap().to_string() } else { val.to_string() }
                } else if let Some(url) = map.get("url") {
                    if url.is_string() { url.as_str().unwrap().to_string() } else { url.to_string() }
                } else if let Some(query) = map.get("query") {
                    if query.is_string() { query.as_str().unwrap().to_string() } else { query.to_string() }
                } else {
                    args.to_string()
                }
            } else {
                args.to_string()
            };

            cmd.arg(input_str);

            let output = cmd.output().map_err(|e| DynamicToolError(format!("Failed to execute python tool: {}", e)))?;
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                Ok(stdout)
            } else {
                Err(DynamicToolError(format!("Tool execution failed: {}\n{}", stdout, stderr)))
            }
        }
        "rust" => {
            match info.name.as_str() {
                "os_tool" => Ok("Native OS Tool called (Placeholder)".into()),
                "vision_analysis" => Ok("Native Vision Analysis called (Placeholder)".into()),
                "charting" => {
                    let input_str = if let Some(input) = args.get("input") {
                        if input.is_string() {
                            input.as_str().unwrap_or("").to_string()
                        } else {
                            input.to_string()
                        }
                    } else {
                        args.to_string()
                    }.trim().to_string(); // Trim whitespace from the final input string

                    // Log the raw input for debugging (it will show up in terminal)
                    println!("📊 CHARTING INPUT: {}", input_str);
                    
                    let req: charting::ChartRequest = serde_json::from_str(&input_str)
                        .or_else(|_| {
                            // If direct parsing as ChartRequest fails, try parsing as a generic JSON Value first.
                            // This handles cases where the agent might provide a JSON object that was then
                            // stringified, and `from_str` expects a direct ChartRequest JSON.
                            // If `input_str` is already a valid JSON object string, `from_str::<serde_json::Value>` will succeed.
                            // Then, `from_value` attempts to convert that generic JSON Value into ChartRequest.
                            let val: serde_json::Value = serde_json::from_str(&input_str)?;
                            serde_json::from_value(val).map_err(|e| e)
                        })
                        .map_err(|e| DynamicToolError(format!("Invalid chart request format: {}. Ensure you provide a JSON object with: {{\"chart_type\": \"bar|line|pie\", \"title\": \"...\", \"labels\": [...], \"data\": [...]}}", e)))?;
                    
                    charting::generate_chart_svg(req)
                        .map_err(|e| DynamicToolError(e))
                }
                _ => Err(DynamicToolError(format!("Native tool '{}' not implemented in this build", info.name))),
            }
        }
        _ => Err(DynamicToolError(format!("Unsupported tool language: {}", info.language))),
    }
}

/// Public entry point for manual tool dispatch from dispatcher.rs
pub async fn call_dynamic_tool(info: &ToolInfo, args: serde_json::Value) -> Result<String, DynamicToolError> {
    call_dynamic_tool_logic(info, args).await
}

// DynamicToolRouter kept for backward compatibility but no longer used in dynamic dispatch
pub struct DynamicToolRouter {
    pub tools: Vec<ToolInfo>,
}

impl DynamicToolRouter {
    pub fn new(tools: Vec<ToolInfo>) -> Self {
        Self { tools }
    }
}

impl Tool for DynamicToolRouter {
    const NAME: &'static str = "execute_tool";
    type Args = serde_json::Value;
    type Output = String;
    type Error = DynamicToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        let tool_list = self.tools.iter()
            .map(|t| format!("{}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        let enum_values = self.tools.iter()
            .map(|t| json!(t.name))
            .collect::<Vec<_>>();

        ToolDefinition {
            name: "execute_tool".to_string(),
            description: format!(
                "Execute specialized tools for this supervisor. Available sub-tools:\n{}",
                tool_list
            ),
            parameters: json!({
                "type": "object",
                "properties": {
                    "tool_name": {
                        "type": "string",
                        "enum": enum_values,
                        "description": "The name of the sub-tool to execute"
                    },
                    "input": {
                        "type": "string",
                        "description": "Main input for the selected tool (e.g. search query or URL)"
                    }
                },
                "required": ["tool_name", "input"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let tool_name = args.get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DynamicToolError("Missing tool_name".into()))?;

        let tool_info = self.tools.iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| DynamicToolError(format!("Tool '{}' not found in this router", tool_name)))?;

        call_dynamic_tool_logic(tool_info, args).await
    }
}

// Structured perception tool — unchanged
#[derive(Clone)]
pub struct StructuredPerceptionTool {
    pub db: crate::db::Database,
}

impl StructuredPerceptionTool {
    pub fn new(db: crate::db::Database) -> Self {
        Self { db }
    }
}