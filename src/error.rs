use crate::pipeline::PipelineError;
use crate::protocol::ProtocolError;
use thiserror::Error;

/// Structured error context for better error handling and debugging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorContext {
    /// Field path or configuration key that caused the error (e.g., "manifest.base_url", "request.messages[0].content")
    pub field_path: Option<String>,
    /// Additional context about the error (e.g., expected type, actual value)
    pub details: Option<String>,
    /// Source of the error (e.g., "protocol_loader", "request_validator")
    pub source: Option<String>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            field_path: None,
            details: None,
            source: None,
        }
    }

    pub fn with_field_path(mut self, path: impl Into<String>) -> Self {
        self.field_path = Some(path.into());
        self
    }

    pub fn with_details(mut self, details: impl Into<String>) -> Self {
        self.details = Some(details.into());
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }
}

impl Default for ErrorContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Unified error type for the AI-Protocol Runtime
/// This aggregates all low-level errors into actionable, high-level categories
#[derive(Debug, Error)]
pub enum Error {
    #[error("Protocol specification error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("Pipeline processing error: {0}")]
    Pipeline(#[from] PipelineError),

    #[error("Configuration error: {message}{}", format_context(.context))]
    Configuration {
        message: String,
        context: ErrorContext,
    },

    #[error("Validation error: {message}{}", format_context(.context))]
    Validation {
        message: String,
        context: ErrorContext,
    },

    #[error("Runtime error: {message}{}", format_context(.context))]
    Runtime {
        message: String,
        context: ErrorContext,
    },

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

    #[error("Unknown error: {message}{}", format_context(.context))]
    Unknown {
        message: String,
        context: ErrorContext,
    },
}

// Helper function to format error context for display
fn format_context(ctx: &ErrorContext) -> String {
    let mut parts = Vec::new();
    if let Some(ref field) = ctx.field_path {
        parts.push(format!("field: {}", field));
    }
    if let Some(ref details) = ctx.details {
        parts.push(format!("details: {}", details));
    }
    if let Some(ref source) = ctx.source {
        parts.push(format!("source: {}", source));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" ({})", parts.join(", "))
    }
}

impl Error {
    /// Create a new runtime error with structured context
    pub fn runtime_with_context(msg: impl Into<String>, context: ErrorContext) -> Self {
        Error::Runtime {
            message: msg.into(),
            context,
        }
    }

    /// Create a new validation error with structured context
    pub fn validation_with_context(msg: impl Into<String>, context: ErrorContext) -> Self {
        Error::Validation {
            message: msg.into(),
            context,
        }
    }

    /// Create a new configuration error with structured context
    pub fn configuration_with_context(msg: impl Into<String>, context: ErrorContext) -> Self {
        Error::Configuration {
            message: msg.into(),
            context,
        }
    }

    /// Create a new unknown error with structured context
    pub fn unknown_with_context(msg: impl Into<String>, context: ErrorContext) -> Self {
        Error::Unknown {
            message: msg.into(),
            context,
        }
    }

    /// Extract error context if available
    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            Error::Configuration { context, .. }
            | Error::Validation { context, .. }
            | Error::Runtime { context, .. }
            | Error::Unknown { context, .. } => Some(context),
            _ => None,
        }
    }
}

// Re-export specific error types for convenience
pub use crate::pipeline::PipelineError as Pipeline;
pub use crate::protocol::ProtocolError as Protocol;
