//! Kernel error type.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum KernelError {
    #[error("database error: {0}")]
    Db(#[from] sqlx::Error),

    /// The approval id is unknown or was already resolved (duplicate resolve).
    #[error("approval '{0}' is not pending (unknown or already resolved)")]
    ApprovalNotPending(String),

    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type KernelResult<T> = Result<T, KernelError>;
