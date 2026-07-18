pub mod conversations;
pub mod messages;
pub mod agents;
pub mod workflows;
pub mod misc;

pub use conversations::ConversationClient;
pub use messages::MessageClient;
pub use agents::{AgentClient, ToolClient, SupervisorClient};
pub use workflows::WorkflowClient;
pub use misc::{
    SettingsClient, SupervisionClient, RegistryClient, SystemClient,
    WorkItemClient, ArtifactClient, PlannerClient, VersionClient,
    OrchestrateClient,
};

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
{}
