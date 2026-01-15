//! Error classification logic

/// Determine if an error class is fallbackable based on protocol specification.
///
/// This follows the standard error_classes from spec.yaml:
/// - Transient errors (retryable) are typically fallbackable
/// - Quota/authentication errors may be fallbackable if another provider is available
/// - Invalid requests are NOT fallbackable (they'll fail on any provider)
pub(crate) fn is_fallbackable_error_class(error_class: &str) -> bool {
    // Based on spec.yaml standard_schema.error_handling.error_classes:
    // Transient errors (default_retryable: true) are typically fallbackable
    match error_class {
        // Transient server errors - fallback makes sense
        "rate_limited" | "overloaded" | "server_error" | "timeout" | "conflict" => true,
        // Quota exhausted - may work on another provider
        "quota_exhausted" => true,
        // Client errors - don't fallback (will fail on any provider)
        "invalid_request" | "authentication" | "permission_denied" | "not_found"
        | "request_too_large" | "cancelled" => false,
        // Unknown/other - conservative: don't fallback
        _ => false,
    }
}
