pub mod agents;
pub mod conversations;
pub mod messages;
pub mod misc;
pub mod workflows;

pub use agents::{AgentClient, SupervisorClient, ToolClient};
pub use conversations::ConversationClient;
pub use messages::MessageClient;
pub use misc::{
    ArtifactClient, OrchestrateClient, PlannerClient, RegistryClient, SettingsClient,
    SupervisionClient, SystemClient, VersionClient, WorkItemClient,
};
pub use workflows::WorkflowClient;

#[async_trait::async_trait]
pub trait BackendClient:
    ConversationClient
    + MessageClient
    + AgentClient
    + ToolClient
    + SupervisorClient
    + WorkflowClient
    + SettingsClient
    + SupervisionClient
    + RegistryClient
    + SystemClient
    + WorkItemClient
    + ArtifactClient
    + PlannerClient
    + VersionClient
    + OrchestrateClient
    + Send
    + Sync
{
}
