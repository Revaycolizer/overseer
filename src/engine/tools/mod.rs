use rig::completion::ToolDefinition;
use rig::tool::Tool;
use serde_json::json;
use tokio::process::Command;

use crate::engine::utils::discovery::{build_python_command, get_tool_executable_path, ToolInfo};

pub mod charting;

#[derive(Debug, Clone)]
pub enum DynamicToolError {
    MissingExecutable { tool: String },
    MissingPython { details: String },
    InvalidChartRequest(String),
    UnsupportedLanguage(String),
    NativeNotImplemented(String),
    ExecutionFailed { tool: String, details: String },
    TimedOut { tool: String, timeout_ms: u64 },
}

impl std::fmt::Display for DynamicToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingExecutable { tool } => write!(
                f,
                "No executable path could be resolved for tool '{}'.",
                tool
            ),
            Self::MissingPython { details } => write!(f, "Python runtime unavailable: {}", details),
            Self::InvalidChartRequest(details) => {
                write!(f, "Invalid chart request format: {}", details)
            }
            Self::UnsupportedLanguage(language) => {
                write!(f, "Unsupported tool language: {}", language)
            }
            Self::NativeNotImplemented(tool) => write!(
                f,
                "Native tool '{}' is not implemented in this build.",
                tool
            ),
            Self::ExecutionFailed { tool, details } => {
                write!(f, "Tool '{}' failed: {}", tool, details)
            }
            Self::TimedOut { tool, timeout_ms } => {
                write!(f, "Tool '{}' timed out after {} ms.", tool, timeout_ms)
            }
        }
    }
}

impl std::error::Error for DynamicToolError {}

async fn call_dynamic_tool_logic(
    info: &ToolInfo,
    args: serde_json::Value,
) -> Result<String, DynamicToolError> {
    match info.language.as_str() {
        "python" => {
            let executable_path = get_tool_executable_path(info).ok_or_else(|| {
                DynamicToolError::MissingExecutable {
                    tool: info.name.clone(),
                }
            })?;

            let parts = build_python_command()
                .map_err(|details| DynamicToolError::MissingPython { details })?;
            let mut cmd = Command::new(&parts[0]);
            if parts.len() > 1 {
                cmd.args(&parts[1..]);
            }
            cmd.arg(&executable_path);
            cmd.kill_on_drop(true);

            let input_str = if let Some(input) = args.get("input") {
                if input.is_string() {
                    input.as_str().unwrap_or("").to_string()
                } else {
                    input.to_string()
                }
            } else if let Some(inner) = args.get("args") {
                inner.to_string()
            } else if args.is_object() {
                let map = args.as_object().unwrap();
                if map.len() == 1 {
                    let val = map.values().next().unwrap();
                    if val.is_string() {
                        val.as_str().unwrap().to_string()
                    } else {
                        val.to_string()
                    }
                } else if let Some(url) = map.get("url") {
                    if url.is_string() {
                        url.as_str().unwrap().to_string()
                    } else {
                        url.to_string()
                    }
                } else if let Some(query) = map.get("query") {
                    if query.is_string() {
                        query.as_str().unwrap().to_string()
                    } else {
                        query.to_string()
                    }
                } else {
                    args.to_string()
                }
            } else {
                args.to_string()
            };

            cmd.arg(input_str);

            let output = cmd
                .output()
                .await
                .map_err(|e| DynamicToolError::ExecutionFailed {
                    tool: info.name.clone(),
                    details: e.to_string(),
                })?;

            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();

            if output.status.success() {
                Ok(stdout)
            } else {
                Err(DynamicToolError::ExecutionFailed {
                    tool: info.name.clone(),
                    details: format!("{}\n{}", stdout, stderr).trim().to_string(),
                })
            }
        }
        "rust" => match info.name.as_str() {
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
                }
                .trim()
                .to_string();

                println!("📊 CHARTING INPUT: {}", input_str);

                let req: charting::ChartRequest = serde_json::from_str(&input_str)
                    .or_else(|_| {
                        let val: serde_json::Value = serde_json::from_str(&input_str)?;
                        serde_json::from_value(val)
                    })
                    .map_err(|e| {
                        DynamicToolError::InvalidChartRequest(format!(
                            "{}. Provide {{\"chart_type\":\"bar|line|pie\",\"title\":\"...\",\"labels\":[...],\"data\":[...]}}",
                            e
                        ))
                    })?;

                charting::generate_chart_svg(req).map_err(|details| {
                    DynamicToolError::ExecutionFailed {
                        tool: info.name.clone(),
                        details,
                    }
                })
            }
            _ => Err(DynamicToolError::NativeNotImplemented(info.name.clone())),
        },
        _ => Err(DynamicToolError::UnsupportedLanguage(info.language.clone())),
    }
}

pub async fn call_dynamic_tool(
    info: &ToolInfo,
    args: serde_json::Value,
) -> Result<String, DynamicToolError> {
    call_dynamic_tool_logic(info, args).await
}

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
        let tool_list = self
            .tools
            .iter()
            .map(|t| format!("{}: {}", t.name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        let enum_values = self.tools.iter().map(|t| json!(t.name)).collect::<Vec<_>>();

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
        let tool_name = args
            .get("tool_name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| DynamicToolError::ExecutionFailed {
                tool: "execute_tool".to_string(),
                details: "Missing tool_name".to_string(),
            })?;

        let tool_info = self
            .tools
            .iter()
            .find(|t| t.name == tool_name)
            .ok_or_else(|| DynamicToolError::ExecutionFailed {
                tool: tool_name.to_string(),
                details: "Tool not found in this router".to_string(),
            })?;

        call_dynamic_tool_logic(tool_info, args).await
    }
}

#[derive(Clone)]
pub struct StructuredPerceptionTool {
    pub db: crate::db::Database,
}

impl StructuredPerceptionTool {
    pub fn new(db: crate::db::Database) -> Self {
        Self { db }
    }
}
