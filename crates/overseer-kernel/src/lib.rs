//! # Overseer Kernel (v0.1)
//!
//! Deterministic policy enforcement, approval persistence and runtime
//! coordination for agent actions.
//!
//! Every native tool, Python tool, shell command or computer-control action an
//! agent proposes is converted into an [`action::ActionRequest`] and taken
//! through [`kernel::Kernel::authorize_and_execute`], which:
//!
//! 1. records an `action_requested` trace event,
//! 2. runs the **deterministic** [`policy::evaluate`] (never an LLM),
//! 3. produces exactly one of allow / deny / require-approval,
//! 4. on deny: never executes and returns a structured denial,
//! 5. on require-approval: persists an approval, pauses on an async channel and
//!    waits for a real human decision (no busy-loop),
//! 6. on approval: executes the **immutable** approved payload,
//! 7. on success: increments the enforced action counter,
//! 8. emergency stop cancels waiting approvals and aborts in-flight execution.
//!
//! The crate has no GUI/desktop dependencies so it can be built and tested
//! independently of the Dioxus UI.

pub mod action;
pub mod approval;
pub mod error;
pub mod events;
pub mod kernel;
pub mod policy;

pub use action::{ActionKind, ActionRequest};
pub use approval::{ApprovalRecord, ApprovalStatus, ApprovalStore, APPROVAL_SCHEMA};
pub use error::{KernelError, KernelResult};
pub use events::{KernelEvent, KernelEventKind, TraceSink};
pub use kernel::{ExecutionOutcome, Kernel, Resolution};
pub use policy::{evaluate, PolicyConfig, PolicyDecision, KERNEL_POLICY_VERSION};
