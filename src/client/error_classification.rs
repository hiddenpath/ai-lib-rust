//! 错误分类逻辑：将 HTTP 状态码和提供商错误映射到 V2 标准错误码。
//!
//! Error classification logic.

use crate::error_code::StandardErrorCode;
use serde_yaml::Value;

/// Maps an error class name string to the corresponding AI-Protocol V2 standard error code.
///
/// The class name should match the standard names (e.g., `"invalid_request"`).
/// Aliases such as `"authorized_error"` (→ authentication) are supported.
/// Unknown class names map to `StandardErrorCode::Unknown`.
pub fn classify_to_standard_code(error_class: &str) -> StandardErrorCode {
    StandardErrorCode::from_error_class(error_class)
}

/// Classify an error from HTTP status and optional response body.
///
/// Provider-specific error codes in the response body (e.g., OpenAI `context_length_exceeded`,
/// Anthropic `overloaded_error`) override the HTTP status mapping when available.
pub fn classify_error_from_response(
    http_status: u16,
    response_body: Option<&Value>,
) -> StandardErrorCode {
    if let Some(body) = response_body {
        if let Some(code) = extract_provider_error_code(body) {
            if let Some(std_code) = StandardErrorCode::from_provider_code(&code) {
                return std_code;
            }
        }
    }
    StandardErrorCode::from_http_status(http_status)
}

/// Extract provider error code from response body.
///
/// Handles OpenAI format: `{"error": {"code": "...", "type": "..."}}`
/// and Anthropic format: `{"error": {"type": "..."}}`.
fn extract_provider_error_code(body: &Value) -> Option<String> {
    let mapping = body.as_mapping()?;
    // OpenAI: error.code or error.type
    if let Some(err) = mapping.get("error") {
        if let Some(err_map) = err.as_mapping() {
            if let Some(code) = err_map.get("code").and_then(|v| v.as_str()) {
                return Some(code.to_string());
            }
            if let Some(ty) = err_map.get("type").and_then(|v| v.as_str()) {
                return Some(ty.to_string());
            }
        }
    }
    None
}

/// Determine if an error class is fallbackable based on protocol specification.
///
/// This follows the standard error_classes from spec.yaml:
/// - Transient errors (retryable) are typically fallbackable
/// - Quota/authentication errors are fallbackable (per-provider; another provider may succeed)
/// - Invalid requests are NOT fallbackable (they'll fail on any provider)
pub(crate) fn is_fallbackable_error_class(error_class: &str) -> bool {
    classify_to_standard_code(error_class).fallbackable()
}
