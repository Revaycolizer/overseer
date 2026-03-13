use rig::{
    completion::{ToolDefinition, Prompt, PromptError, Message},
    message::{UserContent, ImageMediaType, DocumentSourceKind},
    tool::Tool,
    client::CompletionClient,
    OneOrMany,
};
use rig::providers::ollama;
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use base64::{Engine as _, engine::general_purpose};

#[derive(Deserialize)]
pub struct ScreenshotArgs {
    pub analyze: Option<bool>,
    pub query: Option<String>,
    pub save_path: Option<String>,
}

pub struct ScreenshotTool {
    pub client: ollama::Client,
}

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

impl ScreenshotTool {
    /// Detect available screenshot tool on the system
    fn detect_screenshot_tool() -> Result<String, ToolError> {
        if cfg!(target_os = "macos") {
            return Ok("screencapture".to_string());
        }
        
        if cfg!(target_os = "windows") {
            return Ok("powershell".to_string());
        }

        let tools = vec!["scrot", "gnome-screenshot", "import", "spectacle"];
        
        for tool in tools {
            if Command::new("which")
                .arg(tool)
                .output()
                .map(|output| output.status.success())
                .unwrap_or(false)
            {
                return Ok(tool.to_string());
            }
        }
        
        Err(ToolError(
            "No screenshot tool found. Please install one of: scrot, gnome-screenshot, imagemagick, or spectacle".to_string()
        ))
    }

    /// Capture screenshot using the detected tool
    fn capture_screenshot(output_path: &Path) -> Result<(), ToolError> {
        let tool = Self::detect_screenshot_tool()?;
        
        let status = match tool.as_str() {
            "screencapture" => {
                // macOS built-in tool
                Command::new("screencapture")
                    .arg("-x") // silent mode
                    .arg(output_path)
                    .status()
            }
            "powershell" => {
                // Windows PowerShell command to capture screen via .NET
                // This script creates a bitmap of the primary screen and saves it.
                let ps_script = format!(
                    "Add-Type -AssemblyName System.Windows.Forms, System.Drawing; \
                     $Screen = [System.Windows.Forms.Screen]::PrimaryScreen; \
                     $Bitmap = New-Object System.Drawing.Bitmap($Screen.Bounds.Width, $Screen.Bounds.Height); \
                     $Graphics = [System.Drawing.Graphics]::FromImage($Bitmap); \
                     $Graphics.CopyFromScreen($Screen.Bounds.X, $Screen.Bounds.Y, 0, 0, $Screen.Bounds.Size); \
                     $Bitmap.Save('{}', [System.Drawing.Imaging.ImageFormat]::Png); \
                     $Graphics.Dispose(); \
                     $Bitmap.Dispose();",
                    output_path.to_string_lossy()
                );
                
                Command::new("powershell")
                    .arg("-Command")
                    .arg(ps_script)
                    .status()
            }
            "scrot" => {
                Command::new("scrot")
                    .arg(output_path)
                    .status()
            }
            "gnome-screenshot" => {
                Command::new("gnome-screenshot")
                    .arg("-f")
                    .arg(output_path)
                    .status()
            }
            "import" => {
                Command::new("import")
                    .arg("-window")
                    .arg("root")
                    .arg(output_path)
                    .status()
            }
            "spectacle" => {
                Command::new("spectacle")
                    .arg("-b")
                    .arg("-n")
                    .arg("-o")
                    .arg(output_path)
                    .status()
            }
            _ => {
                return Err(ToolError(format!("Unsupported screenshot tool: {}", tool)));
            }
        };

        match status {
            Ok(exit_status) if exit_status.success() => Ok(()),
            Ok(exit_status) => Err(ToolError(format!(
                "Screenshot command failed with exit code: {:?}",
                exit_status.code()
            ))),
            Err(e) => Err(ToolError(format!("Failed to execute screenshot command: {}", e))),
        }
    }
}

impl Tool for ScreenshotTool {
    const NAME: &'static str = "take_screenshot";

    type Args = ScreenshotArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Capture a screenshot of the current screen. Works on Windows, macOS, and Linux. Optionally analyze it with AI vision.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "analyze": {
                        "type": "boolean",
                        "description": "Whether to analyze the screenshot with AI vision (default: false)"
                    },
                    "query": {
                        "type": "string",
                        "description": "If analyzing, what question to ask about the screenshot"
                    },
                    "save_path": {
                        "type": "string",
                        "description": "Optional custom path to save the screenshot (default: ~/screenshots/screenshot_<timestamp>.png)"
                    }
                },
                "required": []
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        // Determine save path
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S");
        
        let home_dir = if cfg!(target_os = "windows") {
            std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME"))
        } else {
            std::env::var("HOME")
        }.unwrap_or_else(|_| "/tmp".to_string());

        let default_dir = PathBuf::from(home_dir).join("screenshots");
        
        // Create screenshots directory if it doesn't exist
        std::fs::create_dir_all(&default_dir)
            .map_err(|e| ToolError(format!("Failed to create screenshots directory: {}", e)))?;

        let save_path = if let Some(custom_path) = args.save_path {
            let expanded = if custom_path.starts_with("~/") {
                if let Ok(home) = std::env::var("HOME").or_else(|_| std::env::var("USERPROFILE")) {
                    custom_path.replacen("~", &home, 1)
                } else {
                    custom_path
                }
            } else {
                custom_path
            };
            PathBuf::from(expanded)
        } else {
            default_dir.join(format!("screenshot_{}.png", timestamp))
        };

        // Capture the screenshot
        Self::capture_screenshot(&save_path)?;

        // Verify the file was created
        if !save_path.exists() {
            return Err(ToolError(format!(
                "Screenshot was not saved to {}",
                save_path.display()
            )));
        }

        let mut response = format!("Screenshot saved to: {}", save_path.display());

        // If analysis is requested, analyze the screenshot
        if args.analyze.unwrap_or(false) {
            let query = args.query.unwrap_or_else(|| "Describe what you see in this screenshot.".to_string());
            
            // Read the screenshot file
            let bytes = std::fs::read(&save_path)
                .map_err(|e| ToolError(format!("Failed to read screenshot for analysis: {}", e)))?;
            
            // Encode to base64
            let b64 = general_purpose::STANDARD.encode(&bytes);
            
            // Create vision analysis request
            let image_content = UserContent::Image(rig::completion::message::Image {
                data: DocumentSourceKind::Base64(b64),
                media_type: Some(ImageMediaType::PNG),
                detail: None,
                additional_params: None,
            });

            let message = Message::User {
                content: OneOrMany::many(vec![
                    UserContent::text(format!("Instruction: {}. Analyze the following screenshot.", query)),
                    image_content,
                ])
                .map_err(|e: rig::one_or_many::EmptyListError| ToolError(e.to_string()))?,
            };

            let agent = self.client.agent("qwen3-vl:2b").build();
            let analysis = agent.prompt(message).await?;
            
            response.push_str(&format!("\n\nAnalysis:\n{}", analysis));
        }

        Ok(response)
    }
}
