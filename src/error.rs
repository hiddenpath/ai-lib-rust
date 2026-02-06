//! 错误处理模块：提供统一的错误类型和结构化错误上下文。
//!
//! # Error Handling Module
//!
//! This module provides unified error types and structured error contexts for
//! comprehensive error handling across the ai-lib-rust library.
//!
//! ## Overview
//!
//! The error system provides:
//! - **Unified Error Type**: Single [`Error`] enum for all error conditions
//! - **Structured Context**: Rich [`ErrorContext`] for debugging information
//! - **Actionable Hints**: User-friendly suggestions for error resolution
//! - **Error Classification**: Retryable and fallbackable error marking
//!
//! ## Error Categories
//!
//! | Variant | Description |
//! |---------|-------------|
//! | `Protocol` | Protocol specification errors |
//! | `Pipeline` | Streaming pipeline errors |
//! | `Configuration` | Configuration and setup errors |
//! | `Validation` | Input validation errors |
//! | `Runtime` | Runtime execution errors |
//! | `Transport` | Network transport errors |
//! | `Remote` | Remote API errors (with HTTP status) |
//!
//! ## Example
//!
//! ```rust
//! use ai_lib_rust::error::{Error, ErrorContext};
//!
//! // Create error with context
//! let error = Error::validation_with_context(
//!     "Invalid temperature value",
//!     ErrorContext::new()
//!         .with_field_path("request.temperature")
//!         .with_details("Value must be between 0.0 and 2.0")
//!         .with_hint("Try setting temperature to 0.7 for balanced output"),
//! );
//! ```

use crate::pipeline::PipelineError;
use crate::protocol::ProtocolError;
use thiserror::Error;

/// Structured error context for better error handling and debugging.
///
/// Provides rich metadata about errors including field paths, details,
/// hints, and operational flags for retry/fallback decisions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorContext {
    /// Field path or configuration key that caused the error
    /// (e.g., "manifest.base_url", "request.messages\[0\].content")
    pub field_path: Option<String>,
    /// Additional context about the error (e.g., expected type, actual value)
    pub details: Option<String>,
    /// Source of the error (e.g., "protocol_loader", "request_validator")
    pub source: Option<String>,
    /// Actionable hint or suggestion for the user
    pub hint: Option<String>,
    /// Request identifiers for tracking
    pub request_id: Option<String>,
    /// HTTP status code if applicable
    pub status_code: Option<u16>,
    /// Provider-specific error code
    pub error_code: Option<String>,
    /// Flag indicating if the error is retryable
    pub retryable: Option<bool>,
    /// Flag indicating if the error should trigger a fallback
    pub fallbackable: Option<bool>,
}

impl ErrorContext {
    pub fn new() -> Self {
        Self {
            field_path: None,
            details: None,
            source: None,
            hint: None,
            request_id: None,
            status_code: None,
            error_code: None,
            retryable: None,
            fallbackable: None,
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

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }

    pub fn with_status_code(mut self, code: u16) -> Self {
        self.status_code = Some(code);
        self
    }

    pub fn with_error_code(mut self, code: impl Into<String>) -> Self {
        self.error_code = Some(code.into());
        self
    }

    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = Some(retryable);
        self
    }

    pub fn with_fallbackable(mut self, fallbackable: bool) -> Self {
        self.fallbackable = Some(fallbackable);
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

    #[error("Remote error: HTTP {status} ({class}): {message}{}", format_optional_context(.context))]
    Remote {
        status: u16,
        class: String,
        message: String,
        retryable: bool,
        fallbackable: bool,
        retry_after_ms: Option<u32>,
        context: Option<ErrorContext>,
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
    if let Some(ref id) = ctx.request_id {
        parts.push(format!("request_id: {}", id));
    }
    if let Some(code) = ctx.status_code {
        parts.push(format!("status: {}", code));
    }
    if let Some(ref code) = ctx.error_code {
        parts.push(format!("error_code: {}", code));
    }
    if let Some(retryable) = ctx.retryable {
        parts.push(format!("retryable: {}", retryable));
    }
    if let Some(fallbackable) = ctx.fallbackable {
        parts.push(format!("fallbackable: {}", fallbackable));
    }

    let ctx_str = if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(", "))
    };

    if let Some(ref hint) = ctx.hint {
        format!("{}\n💡 Hint: {}", ctx_str, hint)
    } else {
        ctx_str
    }
}

fn format_optional_context(ctx: &Option<ErrorContext>) -> String {
    ctx.as_ref().map(format_context).unwrap_or_default()
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

    /// Create a simple validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::validation_with_context(msg, ErrorContext::new())
    }

    /// Create a simple configuration error
    pub fn configuration(msg: impl Into<String>) -> Self {
        Self::configuration_with_context(msg, ErrorContext::new())
    }

    /// Create a network error with context
    pub fn network_with_context(msg: impl Into<String>, context: ErrorContext) -> Self {
        Error::Runtime {
            message: format!("Network error: {}", msg.into()),
            context,
        }
    }

    /// Create an API error with context
    pub fn api_with_context(msg: impl Into<String>, context: ErrorContext) -> Self {
        Error::Runtime {
            message: format!("API error: {}", msg.into()),
            context,
        }
    }

    /// Create a parsing error
    pub fn parsing(msg: impl Into<String>) -> Self {
        Error::Validation {
            message: format!("Parsing error: {}", msg.into()),
            context: ErrorContext::new().with_source("parsing"),
        }
    }

    /// Extract error context if available
    pub fn context(&self) -> Option<&ErrorContext> {
        match self {
            Error::Configuration { context, .. }
            | Error::Validation { context, .. }
            | Error::Runtime { context, .. }
            | Error::Unknown { context, .. } => Some(context),
            Error::Remote {
                context: Some(ref c),
                ..
            } => Some(c),
            _ => None,
        }
    }

    /// Attach or update context to the error
    pub fn with_context(mut self, new_ctx: ErrorContext) -> Self {
        match &mut self {
            Error::Configuration {
                ref mut context, ..
            }
            | Error::Validation {
                ref mut context, ..
            }
            | Error::Runtime {
                ref mut context, ..
            }
            | Error::Unknown {
                ref mut context, ..
            } => {
                *context = new_ctx;
            }
            Error::Remote {
                ref mut context, ..
            } => {
                *context = Some(new_ctx);
            }
            _ => {}
        }
        self
    }
}

// Re-export specific error types for convenience
pub use crate::pipeline::PipelineError as Pipeline;
pub use crate::protocol::ProtocolError as Protocol;
