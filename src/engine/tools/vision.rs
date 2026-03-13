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
use std::path::Path;
use base64::{Engine as _, engine::general_purpose};

#[derive(Deserialize)]
pub struct VisionArgs {
    pub query: String,
    pub image_url: String,
}

pub struct VisionTool {
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

impl Tool for VisionTool {
    const NAME: &'static str = "vision_analysis";

    type Args = VisionArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "Analyze images and perform OCR. Supports URLs (http/https) and local file paths.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Specific question or instruction about the image content"
                    },
                    "image_url": {
                        "type": "string",
                        "description": "The URL or local path (e.g., ~/Pictures/img.png) to the image"
                    }
                },
                "required": ["query", "image_url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let agent = self.client.agent("llava:7b").build();

        // 1. Resolve path/URL to bytes and mime type
        let (bytes, mime_str) = if args.image_url.starts_with("http") {
            // URL Fetching
            let resp = reqwest::get(&args.image_url)
                .await
                .map_err(|e| ToolError(format!("Failed to fetch remote image: {}", e)))?;
            
            let mime = resp.headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .unwrap_or("image/jpeg")
                .to_string();
                
            let b = resp.bytes()
                .await
                .map_err(|e| ToolError(format!("Failed to read remote image content: {}", e)))?;
            
            (b.to_vec(), mime)
        } else {
            // Local file handling
            let expanded_path = if args.image_url.starts_with("~/") {
                if let Some(home) = std::env::var_os("HOME") {
                    args.image_url.replacen("~", home.to_string_lossy().as_ref(), 1)
                } else {
                    args.image_url.clone()
                }
            } else {
                args.image_url.clone()
            };

            let path = Path::new(&expanded_path);
            if !path.exists() {
                return Err(ToolError(format!("Local file not found: {}", expanded_path)));
            }

            let b = std::fs::read(path).map_err(|e| ToolError(format!("Failed to read local file: {}", e)))?;
            
            // Basic format verification: check for AVIF signature (ftypavif) if it fails later
            // or just check it now to give a better error.
            if b.len() > 12 && &b[4..12] == b"ftypavif" {
                 return Err(ToolError("The file is an AVIF image, which is currently not supported by the local vision backend. Please provide a PNG or JPEG file.".to_string()));
            }

            let mime = mime_guess::from_path(path).first_or_octet_stream().to_string();
            
            (b, mime)
        };

        // 2. Encode to base64
        let b64 = general_purpose::STANDARD.encode(&bytes);
        
        // 3. Map media type
        let media_type = match mime_str.as_str() {
            "image/png" => Some(ImageMediaType::PNG),
            "image/jpeg" | "image/jpg" => Some(ImageMediaType::JPEG),
            "image/webp" => Some(ImageMediaType::WEBP),
            "image/gif" => Some(ImageMediaType::GIF),
            unsupported => {
                if unsupported.contains("avif") {
                    return Err(ToolError("AVIF format is currently experimental in many local LLM backends. Please provide a PNG or JPG if this fails.".to_string()));
                }
                None
            }
        };

        let image_content = UserContent::Image(rig::completion::message::Image {
            data: DocumentSourceKind::Base64(b64),
            media_type,
            detail: None,
            additional_params: None,
        });

        // 4. Construct a multimodal message
        let message = Message::User {
            content: OneOrMany::many(vec![
                UserContent::text(format!("Instruction: {}. Analyze the following image data directy.", args.query)),
                image_content,
            ]).map_err(|e: rig::one_or_many::EmptyListError| ToolError(e.to_string()))?,
        };

        let response = agent
            .prompt(message)
            .await?;

        Ok(response)
    }
}
