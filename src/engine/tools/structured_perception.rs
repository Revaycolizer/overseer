use rig::{
    completion::ToolDefinition,
    tool::Tool,
};
use serde_json::json;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use std::process::Command;

#[derive(Deserialize)]
pub struct PerceptionArgs {
    pub action: String,
    pub app: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UIElement {
    pub text: String,
    pub role: String,
    pub path: String,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub visibility: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PerceptionResponse {
    pub elements: Vec<UIElement>,
    pub visual_hash: Option<String>,
}

#[derive(Clone)]
pub struct StructuredPerceptionTool {
    last_state: Arc<Mutex<Option<PerceptionResponse>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolError(String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolError {}

impl StructuredPerceptionTool {
    pub fn new() -> Self {
        Self {
            last_state: Arc::new(Mutex::new(None)),
        }
    }

    fn call_python(&self, action: &str, app: &str) -> Result<String, ToolError> {
        let output = Command::new("python3")
            .arg("src/engine/scripts/desktop/ui_automation.py")
            .arg("--action")
            .arg(action)
            .arg("--app")
            .arg(app)
            .output()
            .map_err(|e| ToolError(format!("Failed to execute python: {}", e)))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(ToolError(format!("Python error: {}", err)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    fn get_hash(&self, response: &PerceptionResponse) -> String {
        if let Some(vh) = &response.visual_hash {
            if vh != "00000000000000000000000000000000" && !vh.is_empty() {
                return vh.clone();
            }
        }
        
        let signature = response.elements.iter()
            .map(|e| format!("{}:{}:{:?}", e.role, e.text, e.visibility))
            .collect::<Vec<_>>()
            .join("|");
        format!("{:x}", md5::compute(signature))
    }
}

impl Tool for StructuredPerceptionTool {
    const NAME: &'static str = "structured_perception";
    type Args = PerceptionArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Production-grade structured UI perception using the Sidecar Perception Engine.
            Provides a pixel-perfect 'Eye' by querying the Linux Accessibility Bus AND comparing visual screenshots.
            actions:
            - 'get_ui_tree': app='name' (Returns a filtered JSON list of visible elements + visual hash)
            - 'get_delta': app='name' (Compares current state with last known state to detect transitions)
            - 'get_screen_text': app='name' (Fallback: Returns JSON list of text blocks via OCR)
            
            Always prefer this over OCR for desktop apps and browsers.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "enum": ["get_ui_tree", "get_delta", "wait_for_settle", "get_screen_text"]
                    },
                    "app": {
                        "type": "string",
                        "description": "Application name or window title to focus"
                    }
                },
                "required": ["action"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let app_name = args.app.as_deref().unwrap_or("active");

        match args.action.as_str() {
            "get_ui_tree" => {
                let stdout = self.call_python("get_ui_tree", app_name)?;
                
                // Parse either legacy list or new dict
                let response: PerceptionResponse = if let Ok(list) = serde_json::from_str::<Vec<UIElement>>(&stdout) {
                     PerceptionResponse { elements: list, visual_hash: None }
                } else {
                     serde_json::from_str(&stdout)
                        .map_err(|e| ToolError(format!("Failed to parse JSON: {}", e)))?
                };
                
                let mut last_state = self.last_state.lock().await;
                *last_state = Some(response.clone());
                
                // Return just elements to the Agent (keep it clean)
                Ok(serde_json::to_string(&response.elements).unwrap())
            },
            "wait_for_settle" => {
                let mut last_hash = String::new();
                let mut stable_since = std::time::Instant::now();
                let start_time = std::time::Instant::now();
                let timeout = std::time::Duration::from_secs(10);
                let stabilization = std::time::Duration::from_secs(2);

                while start_time.elapsed() < timeout {
                    let stdout = match self.call_python("get_ui_tree", app_name) {
                        Ok(out) => out,
                        Err(_) => continue, 
                    };
                    
                    let response: PerceptionResponse = if let Ok(list) = serde_json::from_str::<Vec<UIElement>>(&stdout) {
                         PerceptionResponse { elements: list, visual_hash: None }
                    } else {
                         serde_json::from_str(&stdout).unwrap_or(PerceptionResponse { elements: vec![], visual_hash: None })
                    };
                    
                    let current_hash = self.get_hash(&response);
                    
                    if current_hash != last_hash {
                        last_hash = current_hash;
                        stable_since = std::time::Instant::now();
                    } else if stable_since.elapsed() >= stabilization {
                        return Ok(json!({"status": "Settled", "element_count": response.elements.len()}).to_string());
                    }
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                Ok(json!({"status": "Timeout", "message": "UI did not settle"}).to_string())
            },
            "get_delta" => {
                let stdout = self.call_python("get_ui_tree", app_name)?;
                let new_state: PerceptionResponse = if let Ok(list) = serde_json::from_str::<Vec<UIElement>>(&stdout) {
                     PerceptionResponse { elements: list, visual_hash: None }
                } else {
                     serde_json::from_str(&stdout).map_err(|e| ToolError(format!("Failed to parse JSON: {}", e)))?
                };
                
                let last_state_guard = self.last_state.lock().await;
                
                if let Some(old_state) = last_state_guard.as_ref() {
                    // Visual Hash check
                    let visual_change = if let (Some(h1), Some(h2)) = (&old_state.visual_hash, &new_state.visual_hash) {
                        h1 != h2 && h1 != "00000000000000000000000000000000" && h2 != "00000000000000000000000000000000"
                    } else {
                        false
                    };

                    let old_set: std::collections::HashSet<String> = old_state.elements.iter().map(|e| format!("{}:{}", e.role, e.text)).collect();
                    let new_set: std::collections::HashSet<String> = new_state.elements.iter().map(|e| format!("{}:{}", e.role, e.text)).collect();
                    
                    let added: Vec<_> = new_state.elements.iter()
                        .filter(|e| !old_set.contains(&format!("{}:{}", e.role, e.text)))
                        .cloned()
                        .collect();
                        
                    let removed_count = old_state.elements.iter()
                        .filter(|e| !new_set.contains(&format!("{}:{}", e.role, e.text)))
                        .count();
                    
                    // If visual hash changed, it IS a change, even if elements are stable.
                    let status = if !added.is_empty() || removed_count > 0 || visual_change { "Changed" } else { "Stable" };
                        
                    Ok(json!({
                        "status": status,
                        "visual_change": visual_change,
                        "added_count": added.len(),
                        "added_preview": added.iter().take(5).collect::<Vec<_>>(),
                        "removed_count": removed_count
                    }).to_string())
                } else {
                    Ok(json!({"status": "No Previous State", "message": "Run get_ui_tree first"}).to_string())
                }
            },
            "get_screen_text" => {
                let stdout = self.call_python("get_screen_text", app_name)?;
                Ok(stdout)
            },
            _ => Err(ToolError("Unknown action".into())),
        }
    }
}
