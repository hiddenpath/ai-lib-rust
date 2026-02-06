//! Protocol error types

/// Protocol error types
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Failed to load protocol from {path}: {reason}{}", .hint.as_ref().map(|h| format!("\n Hint: {}", h)).unwrap_or_default())]
    LoadError {
        path: String,
        reason: String,
        hint: Option<String>,
    },

    #[error("Protocol validation failed: {0}")]
    ValidationError(String),

    #[error("Schema mismatch: expected {expected}, found {actual} at {path}{}", .hint.as_ref().map(|h| format!("\n Hint: {}", h)).unwrap_or_default())]
    SchemaMismatch {
        path: String,
        expected: String,
        actual: String,
        hint: Option<String>,
    },

    #[error("Protocol not found: {id}{}", .hint.as_ref().map(|h| format!("\n Hint: {}", h)).unwrap_or_default())]
    NotFound { id: String, hint: Option<String> },

    #[error("Unsupported protocol version '{version}' (max supported: {max_supported}){}", .hint.as_ref().map(|h| format!("\n Hint: {}", h)).unwrap_or_default())]
    InvalidVersion {
        version: String,
        max_supported: String,
        hint: Option<String>,
    },

    #[error("Configuration manifest error: {0}")]
    ManifestError(String),

    #[error("Internal protocol error: {0}")]
    Internal(String),

    #[error("YAML syntax error: {0}")]
    YamlError(String),
}

impl ProtocolError {
    /// Attach an actionable hint to the error
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        let hint_val = Some(hint.into());
        match self {
            ProtocolError::LoadError { ref mut hint, .. } => *hint = hint_val,
            ProtocolError::SchemaMismatch { ref mut hint, .. } => *hint = hint_val,
            ProtocolError::NotFound { ref mut hint, .. } => *hint = hint_val,
            ProtocolError::InvalidVersion { ref mut hint, .. } => *hint = hint_val,
            _ => (),
        }
        self
    }
}
