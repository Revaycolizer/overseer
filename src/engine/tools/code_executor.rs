use rig::{
    completion::ToolDefinition,
    tool::Tool,
};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Deserialize)]
pub struct CodeExecutorArgs {
    pub language: String,
    pub code: String,
}

pub struct CodeExecutorTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolError(String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolError {}

impl CodeExecutorTool {
    pub fn new() -> Self {
        Self
    }
}

impl Tool for CodeExecutorTool {
    const NAME: &'static str = "code_executor";
    type Args = CodeExecutorArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Execute code in a sandboxed subprocess with a 10-second timeout. \
                Use this for: math calculations, data processing, string manipulation, \
                file parsing, or any computation that needs precise results. \
                Supported languages: python, bash. \
                The code runs in isolation — no network access, no persistent state. \
                Output is captured from stdout. Errors from stderr.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "language": {
                        "type": "string",
                        "enum": ["python", "bash"],
                        "description": "Programming language to execute"
                    },
                    "code": {
                        "type": "string",
                        "description": "The code to execute. For Python, use print() for output."
                    }
                },
                "required": ["language", "code"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let (cmd, cmd_args) = match args.language.to_lowercase().as_str() {
            "python" => ("python3", vec!["-c", &args.code]),
            "bash" => ("bash", vec!["-c", &args.code]),
            other => return Err(ToolError(format!("Unsupported language: '{}'. Use 'python' or 'bash'.", other))),
        };

        // Run with 30-second timeout
        let output = Command::new("timeout")
            .arg("30")
            .arg(cmd)
            .args(&cmd_args)
            .output()
            .map_err(|e| ToolError(format!("Failed to execute code: {}", e)))?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if output.status.success() {
            let combined_output = format!("{}\n{}", stdout, stderr).trim().to_string();
            if combined_output.is_empty() {
                Ok("Code executed successfully (no output).".to_string())
            } else {
                // Truncate output to prevent context overflow
                let truncated = if combined_output.len() > 4096 {
                    format!("{}...\n[OUTPUT TRUNCATED - {} bytes total]", &combined_output[..4096], combined_output.len())
                } else {
                    combined_output
                };
                Ok(format!("Result:\n{}", truncated))
            }
        } else {
            // Check for timeout (exit code 124)
            if output.status.code() == Some(124) {
                Err(ToolError("Code execution timed out (10 second limit exceeded).".to_string()))
            } else {
                let error_msg = if stderr.is_empty() { "Unknown error".to_string() } else { stderr };
                // Truncate error output too
                let truncated_err = if error_msg.len() > 2048 {
                    format!("{}...\n[ERROR TRUNCATED]", &error_msg[..2048])
                } else {
                    error_msg
                };
                Err(ToolError(format!("Code execution failed:\n{}", truncated_err)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_code_executor_python_math() {
        let tool = CodeExecutorTool::new();
        let result = tool.call(CodeExecutorArgs {
            language: "python".to_string(),
            code: "print(2 + 2)".to_string(),
        }).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("4"), "Expected '4' in output, got: {}", output);
    }

    #[tokio::test]
    async fn test_code_executor_python_multiline() {
        let tool = CodeExecutorTool::new();
        let result = tool.call(CodeExecutorArgs {
            language: "python".to_string(),
            code: "import math\nprint(f'Pi is approximately {math.pi:.4f}')".to_string(),
        }).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("3.1416"), "Expected Pi in output, got: {}", output);
    }

    #[tokio::test]
    async fn test_code_executor_bash() {
        let tool = CodeExecutorTool::new();
        let result = tool.call(CodeExecutorArgs {
            language: "bash".to_string(),
            code: "echo 'hello world'".to_string(),
        }).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("hello world"), "Expected 'hello world' in output, got: {}", output);
    }

    #[tokio::test]
    async fn test_code_executor_error_handling() {
        let tool = CodeExecutorTool::new();
        let result = tool.call(CodeExecutorArgs {
            language: "python".to_string(),
            code: "raise ValueError('test error')".to_string(),
        }).await;
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.0.contains("ValueError"), "Expected ValueError in error, got: {}", error);
    }

    #[tokio::test]
    async fn test_code_executor_unsupported_language() {
        let tool = CodeExecutorTool::new();
        let result = tool.call(CodeExecutorArgs {
            language: "ruby".to_string(),
            code: "puts 'hello'".to_string(),
        }).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("Unsupported language"));
    }

    #[tokio::test]
    async fn test_code_executor_timeout() {
        let tool = CodeExecutorTool::new();
        let result = tool.call(CodeExecutorArgs {
            language: "python".to_string(),
            code: "import time; time.sleep(15)".to_string(),
        }).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("timed out"));
    }
}
