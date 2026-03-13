use rig::{
    completion::{ToolDefinition, Prompt, PromptError},
    tool::Tool,
    client::CompletionClient,
};
use rig::providers::ollama;
use serde_json::json;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct GeneralArgs {
    pub query: String,
}

pub struct GeneralTool {
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

impl Tool for GeneralTool {
    const NAME: &'static str = "general_reasoning";

    type Args = GeneralArgs;
    type Output = String;
    type Error = ToolError;

    async fn definition(&self, _prompt: String) -> ToolDefinition {
        ToolDefinition {
            name: Self::NAME.into(),
            description: "General text reasoning".into(),
            parameters: json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The text to reason about"
                    }
                },
                "required": ["query"]
            }),
        }
    }

    async fn call(&self, args: Self::Args) -> Result<Self::Output, Self::Error> {
        let agent = self.client.agent("qwen2.5:7b-instruct").build();

        let response = agent
            .prompt(args.query)
            .await?;

        Ok(response)
    }
}
