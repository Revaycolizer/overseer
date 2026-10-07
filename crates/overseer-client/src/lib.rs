//! # Overseer Client
//!
//! HTTP client for the [Overseer](https://github.com/revaycolizer/overseer) AI orchestration platform.
//!
//! ## Quickstart
//!
//! ```rust,no_run
//! use overseer_client::OverseerClient;
//!
//! # async fn run() -> Result<(), Box<dyn std::error::Error>> {
//! let client = OverseerClient::new("http://localhost:8080")?
//!     .with_token("my-token");
//!
//! // List conversations
//! let conversations = client.list_conversations(10).await?;
//! println!("Conversations: {conversations:#?}");
//!
//! // List available agents
//! let agents = client.list_agents().await?;
//! println!("Agents: {agents:#?}");
//!
//! // Send a message
//! let msg_id = client.send_message(1, "user", "Hello, Overseer!", false).await?;
//! println!("Message ID: {msg_id}");
//! # Ok(())
//! # }
//! ```
//!
//! ## Feature Coverage
//!
//! Typed methods cover the main REST resources; `request_json` supports additional JSON routes:
//!
//! | Category         | Methods |
//! |------------------|---------|
//! | Auth             | `register`, `login`, `logout`, `me` |
//! | Conversations    | `list`, `create`, `update_title`, `delete`, `search` |
//! | Messages         | `list`, `send`, `delete`, `list_attachments`, `add_attachment` |
//! | Agents           | `list`, `list_all`, `create`, `update`, `get_by_domain` |
//! | Tools            | `list`, `create`, `update`, `link`, `unlink` |
//! | Supervisors      | `list`, `create`, `update`, `delete`, `get_tools`, `get_agents`, `link`, `unlink` |
//! | Workflows        | `list`, `create`, `update`, `delete`, `steps`, `runs`, `publish`, `versions` |
//! | Work Items       | `list`, `create`, `update_status`, `update_runtime`, `events` |
//! | Artifacts        | `list`, `create`, `get_by_ids`, `get_by_run` |
//! | Planners         | `list`, `create` |
//! | Supervision      | `state`, `approve`, `reject`, `activate`, `kernel_approvals` |
//! | Version Control  | `publish`, `rollback`, `history` |
//! | Browser          | `sessions`, `navigate`, `screenshot`, `policy`, `audit`, `recording` |
//! | Credentials      | `list`, `create`, `update`, `delete` |
//! | Workflow Templates | `list`, `create`, `install` |
//! | Settings         | `get`, `set`, `get_all` |
//! | System           | `health`, `load`, `models`, `orchestrate`, `registry` |

mod client;
pub mod error;
pub mod types;

pub use client::OverseerClient;
pub use error::ClientError;
pub use types::*;
