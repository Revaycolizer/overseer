//! Error types for the Overseer client.

use thiserror::Error;

/// Errors that can occur when using [`crate::OverseerClient`].
#[derive(Error, Debug)]
pub enum ClientError {
    /// The base URL could not be parsed.
    #[error("invalid base URL: {0}")]
    InvalidUrl(String),

    /// An HTTP transport error (connection, timeout, etc.).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The server returned a non-2xx status with a textual error body.
    #[error("API error (status {status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    /// The response body could not be parsed as the expected type.
    #[error("response parse error: {0}")]
    Parse(String),

    /// No authentication token is configured but the endpoint requires one.
    #[error("authentication required")]
    Unauthorized,
}
