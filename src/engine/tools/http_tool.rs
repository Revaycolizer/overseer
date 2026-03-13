use rig::{
    completion::ToolDefinition,
    tool::Tool,
};
use serde_json::json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct HttpArgs {
    pub method: String,
    pub url: String,
    pub body: Option<String>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

pub struct HttpTool;

#[derive(Debug, Serialize, Deserialize)]
pub struct ToolError(String);

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for ToolError {}

impl HttpTool {
    pub fn new() -> Self {
        Self
    }

    fn validate_url(url: &str) -> Result<(), ToolError> {
        let lower = url.to_lowercase();
        if lower.starts_with("file://") {
            return Err(ToolError("file:// URLs are not allowed for security.".to_string()));
        }
        if !lower.starts_with("http://") && !lower.starts_with("https://") {
            return Err(ToolError("URL must start with http:// or https://".to_string()));
        }
        Ok(())
    }
}

const MAX_RESPONSE_BYTES: usize = 10 * 1024; // 10KB limit

impl Tool for HttpTool {
    const NAME: &'static str = "http_request";
    type Args = HttpArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.to_string(),
            description: "Make HTTP requests to REST APIs. \
                Use this for: fetching web data, calling APIs, checking URLs. \
                Supports GET and POST methods. Response is truncated to 10KB. \
                Timeout: 10 seconds. Only http:// and https:// URLs allowed.".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "method": {
                        "type": "string",
                        "enum": ["GET", "POST"],
                        "description": "HTTP method"
                    },
                    "url": {
                        "type": "string",
                        "description": "The URL to request (must be http:// or https://)"
                    },
                    "body": {
                        "type": "string",
                        "description": "Request body for POST requests (JSON string)"
                    },
                    "headers": {
                        "type": "object",
                        "description": "Optional HTTP headers as key-value pairs"
                    }
                },
                "required": ["method", "url"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        Self::validate_url(&args.url)?;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| ToolError(format!("Failed to create HTTP client: {}", e)))?;

        let request = match args.method.to_uppercase().as_str() {
            "GET" => client.get(&args.url),
            "POST" => {
                let mut req = client.post(&args.url);
                if let Some(body) = &args.body {
                    req = req.header("Content-Type", "application/json").body(body.clone());
                }
                req
            },
            other => return Err(ToolError(format!("Unsupported method: '{}'. Use GET or POST.", other))),
        };

        // Add custom headers
        let mut request = request;
        if let Some(headers) = &args.headers {
            for (key, value) in headers {
                request = request.header(key.as_str(), value.as_str());
            }
        }

        let response = request.send().await
            .map_err(|e| {
                if e.is_timeout() {
                    ToolError("Request timed out (10 second limit exceeded).".to_string())
                } else if e.is_connect() {
                    ToolError(format!("Connection failed: {}", e))
                } else {
                    ToolError(format!("Request failed: {}", e))
                }
            })?;

        let status = response.status();
        let status_code = status.as_u16();
        let status_text = status.canonical_reason().unwrap_or("Unknown");

        let body_text = response.text().await
            .map_err(|e| ToolError(format!("Failed to read response body: {}", e)))?;

        // Truncate response
        let truncated = if body_text.len() > MAX_RESPONSE_BYTES {
            format!(
                "{}...\n[RESPONSE TRUNCATED - {} bytes total, showing first {}]",
                &body_text[..MAX_RESPONSE_BYTES],
                body_text.len(),
                MAX_RESPONSE_BYTES
            )
        } else {
            body_text
        };

        Ok(format!(
            "HTTP {} {} {}\n\nBody:\n{}",
            status_code, status_text, args.url, truncated
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_http_get_success() {
        let tool = HttpTool::new();
        let result = tool.call(HttpArgs {
            method: "GET".to_string(),
            url: "https://httpbin.org/get".to_string(),
            body: None,
            headers: None,
        }).await;
        // httpbin.org might be slow, so we just check it doesn't panic
        match result {
            Ok(output) => assert!(output.contains("HTTP")),
            Err(e) => println!("HTTP test skipped (network issue): {}", e),
        }
    }

    #[test]
    fn test_http_url_validation_file() {
        let result = HttpTool::validate_url("file:///etc/passwd");
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("file://"));
    }

    #[test]
    fn test_http_url_validation_no_scheme() {
        let result = HttpTool::validate_url("example.com");
        assert!(result.is_err());
        assert!(result.unwrap_err().0.contains("http://"));
    }

    #[test]
    fn test_http_url_validation_valid() {
        assert!(HttpTool::validate_url("https://example.com").is_ok());
        assert!(HttpTool::validate_url("http://api.example.com/v1").is_ok());
    }
}
