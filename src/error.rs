use crate::pipeline::PipelineError;
use crate::protocol::ProtocolError;
use thiserror::Error;

/// Unified error type for the AI-Protocol Runtime
/// This aggregates all low-level errors into actionable, high-level categories
#[derive(Debug, Error)]
pub enum Error {
    #[error("Protocol specification error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Pipeline processing error: {0}")]
    Pipeline(#[from] PipelineError),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Network transport error: {0}")]
    Transport(#[from] crate::transport::TransportError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Remote error: HTTP {status} ({class}): {message}")]
    Remote {
        status: u16,
        class: String,
        message: String,
        retryable: bool,
        fallbackable: bool,
        retry_after_ms: Option<u32>,
    },

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl Error {
    /// Create a new runtime error with context
    pub fn runtime(msg: impl Into<String>) -> Self {
        Error::Runtime(msg.into())
    }

    /// Create a new validation error with context
    pub fn validation(msg: impl Into<String>) -> Self {
        Error::Validation(msg.into())
    }
}

// Re-export specific error types for convenience
pub use crate::pipeline::PipelineError as Pipeline;
pub use crate::protocol::ProtocolError as Protocol;
