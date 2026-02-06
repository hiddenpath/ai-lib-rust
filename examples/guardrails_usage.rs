//! Guardrails Usage Example
//!
//! This example demonstrates how to use the Guardrails module for content
//! filtering and safety checks in production applications.
//!
//! Features demonstrated:
//! - Keyword filtering (block, warn, sanitize)
//! - PII detection (emails, phone numbers, credit cards)
//! - Content sanitization
//! - Custom filter rules
//! - Integration with chat requests
//!
//! Usage:
//!   cargo run --example guardrails_usage

use ai_lib_rust::guardrails::{
    FilterAction, FilterRule, Guardrails, GuardrailsConfig,
};
use ai_lib_rust::Message;

fn main() {
    println!("=== AI-Lib Guardrails Demo ===\n");

    // Example 1: Basic keyword filtering
    demo_keyword_filtering();

    // Example 2: PII detection
    demo_pii_detection();

    // Example 3: Content sanitization
    demo_sanitization();

    // Example 4: Custom rules
    demo_custom_rules();

    // Example 5: Message checking
    demo_message_checking();

    // Example 6: Production configuration
    demo_production_config();
}

fn demo_keyword_filtering() {
    println!("--- Example 1: Keyword Filtering ---\n");

    let config = GuardrailsConfig::builder()
        .add_keyword_filter("password", FilterAction::Block)
        .add_keyword_filter("secret", FilterAction::Warn)
        .add_keyword_filter("confidential", FilterAction::Log)
        .build();

    let guardrails = Guardrails::new(config);

    // Test various inputs
    let test_cases = vec![
        "Hello, how are you?",
        "My password is 12345",
        "This is a secret message",
        "This document is confidential",
    ];

    for input in test_cases {
        let result = guardrails.check_input(input);
        println!("Input: {:?}", input);
        println!("Result: {}", result);
        if result.has_violations() {
            for v in result.violations() {
                println!("  - Violation: {} ({:?})", v.pattern, v.action);
            }
        }
        println!();
    }
}

fn demo_pii_detection() {
    println!("--- Example 2: PII Detection ---\n");

    let config = GuardrailsConfig::builder()
        .enable_pii_detection(true)
        .build();

    let guardrails = Guardrails::new(config);

    let test_cases = vec![
        "Contact me at john@example.com",
        "Call me at 555-123-4567",
        "My card number is 4111111111111111",
        "SSN: 123-45-6789",
        "Server IP: 192.168.1.100",
        "No PII in this message",
    ];

    for input in test_cases {
        let result = guardrails.check_input(input);
        println!("Input: {:?}", input);
        println!("Result: {}", result);
        if result.has_violations() {
            for v in result.violations() {
                println!(
                    "  - {} detected: {:?} ({:?})",
                    v.pattern,
                    v.matched_text.as_deref().unwrap_or("N/A"),
                    v.action
                );
            }
        }
        println!();
    }
}

fn demo_sanitization() {
    println!("--- Example 3: Content Sanitization ---\n");

    let config = GuardrailsConfig::builder()
        .add_keyword_filter("secret", FilterAction::Sanitize)
        .add_keyword_filter("password", FilterAction::Sanitize)
        .enable_pii_detection(true)
        .sanitize_replacement("[REDACTED]".to_string())
        .pii_replacement("[PII_REMOVED]".to_string())
        .build();

    let guardrails = Guardrails::new(config);

    let input = "My secret password is 12345. Email me at user@example.com for details.";
    println!("Original: {}", input);

    let sanitized = guardrails.sanitize(input);
    println!("Sanitized: {}", sanitized);
    println!();
}

fn demo_custom_rules() {
    println!("--- Example 4: Custom Rules ---\n");

    // Create custom rules with categories and descriptions
    let api_key_rule = FilterRule::regex(r"[A-Za-z0-9]{32,}", FilterAction::Warn)
        .with_category("security")
        .with_description("Potential API key or token detected");

    let profanity_rule = FilterRule::keyword("badword", FilterAction::Block)
        .case_sensitive(false)
        .with_category("content")
        .with_description("Inappropriate language");

    let config = GuardrailsConfig::builder()
        .add_rule(api_key_rule)
        .add_rule(profanity_rule)
        .build();

    let guardrails = Guardrails::new(config);

    let test_cases = vec![
        "Here is my API key: sk_test_EXAMPLE_NOT_REAL_KEY_12345",
        "This contains a BADWORD in it",
        "Normal message without issues",
    ];

    for input in test_cases {
        let result = guardrails.check_input(input);
        println!("Input: {:?}", input);
        println!("Result: {}", result);
        for v in result.violations() {
            println!(
                "  - Category: {:?}, Description: {:?}",
                v.category, v.description
            );
        }
        println!();
    }
}

fn demo_message_checking() {
    println!("--- Example 5: Message Checking ---\n");

    let config = GuardrailsConfig::builder()
        .add_keyword_filter("password", FilterAction::Block)
        .enable_pii_detection(true)
        .stop_on_first_block(true)
        .build();

    let guardrails = Guardrails::new(config);

    // Check a single message
    let message = Message::user("What is my password?");
    let result = guardrails.check_message(&message);
    println!("Single message check: {}", result);

    // Check multiple messages
    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("Hello!"),
        Message::user("My email is test@example.com"),
    ];

    let result = guardrails.check_messages(&messages);
    println!("Multiple messages check: {}", result);
    println!("Violations found: {}", result.violations().len());
    println!();
}

fn demo_production_config() {
    println!("--- Example 6: Production Configuration ---\n");

    // Use the strict preset for production
    let strict_guardrails = Guardrails::strict();
    println!("Strict mode enabled with:");
    println!("  - Input filtering: enabled");
    println!("  - Output filtering: enabled");
    println!("  - PII detection: enabled");
    println!("  - Stop on first block: enabled");
    println!();

    // Example of checking before sending to API
    let user_input = "Please send the report to admin@company.com";
    let check_result = strict_guardrails.check_input(user_input);

    if check_result.is_blocked() {
        println!("BLOCKED: Cannot send this message to the API");
        println!("Reason: {:?}", check_result.blocking_violations());
    } else if check_result.is_warned() {
        println!("WARNING: Message has potential issues but will proceed");
        println!("Warnings: {:?}", check_result.warning_violations().len());
        // In production, you might log this or ask for confirmation
    } else {
        println!("PASSED: Message is safe to send");
    }

    // Permissive mode for development/testing
    let _permissive = Guardrails::permissive();
    println!("\nPermissive mode: all content allowed (for testing only)");
}
