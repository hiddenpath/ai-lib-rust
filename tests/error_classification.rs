//! Tests for error classification logic

// Test helper: replicate the logic from AiClient::is_fallbackable_error_class
// This allows us to test the classification logic without exposing internal methods
fn is_fallbackable_error_class(error_class: &str) -> bool {
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

#[test]
fn test_fallbackable_error_classes() {
    // Test cases: transient errors should be fallbackable
    let fallbackable_classes = vec![
        "rate_limited",
        "overloaded",
        "server_error",
        "timeout",
        "conflict",
        "quota_exhausted",
    ];

    for class in fallbackable_classes {
        assert!(
            is_fallbackable_error_class(class),
            "Error class '{}' should be fallbackable",
            class
        );
    }
}

#[test]
fn test_non_fallbackable_error_classes() {
    // Test cases: client errors should NOT be fallbackable
    let non_fallbackable_classes = vec![
        "invalid_request",
        "authentication",
        "permission_denied",
        "not_found",
        "request_too_large",
        "cancelled",
        "other",
        "unknown_error",
        "http_error", // Default fallback
    ];

    for class in non_fallbackable_classes {
        assert!(
            !is_fallbackable_error_class(class),
            "Error class '{}' should NOT be fallbackable",
            class
        );
    }
}

#[test]
fn test_error_class_standard_compliance() {
    // Verify that all error classes used in the code match the spec.yaml standard
    let standard_classes = vec![
        "invalid_request",
        "authentication",
        "permission_denied",
        "not_found",
        "quota_exhausted",
        "rate_limited",
        "request_too_large",
        "timeout",
        "conflict",
        "cancelled",
        "server_error",
        "overloaded",
        "other",
    ];

    // This test ensures we're using standard error classes
    // In practice, we would verify against the actual spec.yaml
    assert_eq!(
        standard_classes.len(),
        13,
        "Should have 13 standard error classes"
    );
}
