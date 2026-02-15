//! V2 标准错误码：定义 13 个规范错误码及其重试/回退语义。
//!
//! AI-Protocol V2 standard error codes.
//!
//! This module defines the canonical error code system from the AI-Protocol V2
//! specification (see `ai-protocol/schemas/v2/error-codes.yaml`). Each runtime
//! implements these codes for consistent error handling across providers.
//!
//! ## Error Code Categories
//!
//! | Prefix | Category    | Description                    |
//! |--------|-------------|--------------------------------|
//! | E1xxx  | client      | Request-side errors            |
//! | E2xxx  | rate        | Rate limit and quota errors    |
//! | E3xxx  | server      | Provider-side errors           |
//! | E4xxx  | operational | Lifecycle and state conflicts  |
//! | E9xxx  | unknown     | Catch-all / unclassified       |
//!
//! ## Example
//!
//! ```rust
//! use ai_lib_rust::error_code::StandardErrorCode;
//!
//! let code = StandardErrorCode::from_error_class("rate_limited");
//! assert_eq!(code.code(), "E2001");
//! assert!(code.retryable());
//! assert_eq!(code.category(), "rate");
//! ```

use std::fmt;

/// Standard AI-Protocol V2 error code.
///
/// Each variant corresponds to a canonical error from the specification,
/// with associated metadata: code string, name, retryable, and fallbackable flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum StandardErrorCode {
    /// E1001: Malformed request, invalid parameters, or missing required fields
    InvalidRequest,
    /// E1002: Invalid, expired, or missing API key
    Authentication,
    /// E1003: Valid credentials but insufficient permissions
    PermissionDenied,
    /// E1004: Requested model, endpoint, or resource does not exist
    NotFound,
    /// E1005: Input exceeds context window or API payload size limit
    RequestTooLarge,
    /// E2001: Request rate limit exceeded
    RateLimited,
    /// E2002: Account usage quota or billing limit reached
    QuotaExhausted,
    /// E3001: Internal server error on provider side
    ServerError,
    /// E3002: Provider service temporarily overloaded
    Overloaded,
    /// E3003: Request timed out before response received
    Timeout,
    /// E4001: State conflict (e.g., concurrent modification)
    Conflict,
    /// E4002: Request was cancelled by the client
    Cancelled,
    /// E9999: Error could not be classified
    Unknown,
}

impl StandardErrorCode {
    /// Returns the canonical code string (e.g., `"E1001"`).
    #[inline]
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "E1001",
            Self::Authentication => "E1002",
            Self::PermissionDenied => "E1003",
            Self::NotFound => "E1004",
            Self::RequestTooLarge => "E1005",
            Self::RateLimited => "E2001",
            Self::QuotaExhausted => "E2002",
            Self::ServerError => "E3001",
            Self::Overloaded => "E3002",
            Self::Timeout => "E3003",
            Self::Conflict => "E4001",
            Self::Cancelled => "E4002",
            Self::Unknown => "E9999",
        }
    }

    /// Returns the standard name (e.g., `"invalid_request"`).
    #[inline]
    pub fn name(&self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::Authentication => "authentication",
            Self::PermissionDenied => "permission_denied",
            Self::NotFound => "not_found",
            Self::RequestTooLarge => "request_too_large",
            Self::RateLimited => "rate_limited",
            Self::QuotaExhausted => "quota_exhausted",
            Self::ServerError => "server_error",
            Self::Overloaded => "overloaded",
            Self::Timeout => "timeout",
            Self::Conflict => "conflict",
            Self::Cancelled => "cancelled",
            Self::Unknown => "unknown",
        }
    }

    /// Returns whether this error is retryable by default.
    #[inline]
    pub fn retryable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited
                | Self::ServerError
                | Self::Overloaded
                | Self::Timeout
                | Self::Conflict
        )
    }

    /// Returns whether this error should trigger a fallback to another provider.
    #[inline]
    pub fn fallbackable(&self) -> bool {
        matches!(
            self,
            Self::Authentication
                | Self::RateLimited
                | Self::QuotaExhausted
                | Self::ServerError
                | Self::Overloaded
                | Self::Timeout
        )
    }

    /// Returns the category: `"client"`, `"rate"`, `"server"`, `"operational"`, or `"unknown"`.
    #[inline]
    pub fn category(&self) -> &'static str {
        match self {
            Self::InvalidRequest
            | Self::Authentication
            | Self::PermissionDenied
            | Self::NotFound
            | Self::RequestTooLarge => "client",
            Self::RateLimited | Self::QuotaExhausted => "rate",
            Self::ServerError | Self::Overloaded | Self::Timeout => "server",
            Self::Conflict | Self::Cancelled => "operational",
            Self::Unknown => "unknown",
        }
    }

    /// Maps a provider error code/type string to the corresponding `StandardErrorCode`.
    ///
    /// Supports both standard names (e.g., `"invalid_request"`) and provider-specific
    /// aliases such as `"invalid_api_key"`, `"context_length_exceeded"`, `"overloaded_error"`.
    pub fn from_provider_code(provider_code: &str) -> Option<Self> {
        let code = match provider_code {
            "invalid_request" | "invalid_request_error" => Self::InvalidRequest,
            "authentication" | "authorized_error" | "invalid_api_key" | "authentication_error" => {
                Self::Authentication
            }
            "permission_denied" | "permission_error" => Self::PermissionDenied,
            "not_found" | "model_not_found" => Self::NotFound,
            "request_too_large" | "context_length_exceeded" => Self::RequestTooLarge,
            "rate_limited" | "rate_limit_exceeded" => Self::RateLimited,
            "quota_exhausted" | "insufficient_quota" => Self::QuotaExhausted,
            "server_error" => Self::ServerError,
            "overloaded" | "overloaded_error" => Self::Overloaded,
            "timeout" => Self::Timeout,
            "conflict" => Self::Conflict,
            "cancelled" => Self::Cancelled,
            _ => return None,
        };
        Some(code)
    }

    /// Maps an error class name string to the corresponding `StandardErrorCode`.
    ///
    /// The class name should match the standard names (e.g., `"invalid_request"`).
    /// Aliases such as `"authorized_error"` (→ authentication) are supported.
    /// Unknown class names map to `StandardErrorCode::Unknown`.
    pub fn from_error_class(error_class: &str) -> Self {
        Self::from_provider_code(error_class).unwrap_or(Self::Unknown)
    }

    /// Maps an HTTP status code to the most likely `StandardErrorCode`.
    ///
    /// Multiple status codes can map to the same error (e.g., 429 → rate_limited).
    /// Status codes without a standard mapping return `StandardErrorCode::Unknown`.
    pub fn from_http_status(status: u16) -> Self {
        match status {
            400 => Self::InvalidRequest,
            401 => Self::Authentication,
            403 => Self::PermissionDenied,
            404 => Self::NotFound,
            408 => Self::Timeout,
            409 => Self::Conflict,
            413 => Self::RequestTooLarge,
            429 => Self::RateLimited, // Could also be QuotaExhausted; default to rate_limited
            500 => Self::ServerError,
            503 => Self::Overloaded,
            504 => Self::Timeout,
            529 => Self::Overloaded, // Anthropic overloaded; non-standard but commonly used
            _ => Self::Unknown,
        }
    }
}

impl fmt::Display for StandardErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}
