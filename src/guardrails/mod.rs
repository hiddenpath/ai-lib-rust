//! 内容安全模块：提供可配置的内容过滤和敏感信息检测功能。
//!
//! # Guardrails Module
//!
//! This module provides configurable content filtering and safety mechanisms
//! to ensure compliant and safe interactions with AI models.
//!
//! ## Overview
//!
//! Guardrails are essential for production AI applications to:
//! - Prevent sensitive information from being sent to AI providers
//! - Filter inappropriate or policy-violating content in responses
//! - Detect and redact personally identifiable information (PII)
//! - Enforce content policies through configurable rules
//!
//! ## Key Features
//!
//! | Feature | Description |
//! |---------|-------------|
//! | **Keyword filtering** | Block or flag messages containing specific keywords |
//! | **Pattern matching** | Use regex patterns to detect sensitive content |
//! | **PII detection** | Detect emails, phone numbers, SSNs, credit cards |
//! | **Content classification** | Categorize content for policy enforcement |
//! | **Configurable actions** | Block, warn, log, or sanitize detected content |
//!
//! ## Components
//!
//! | Component | Description |
//! |-----------|-------------|
//! | [`Guardrails`] | Main controller for applying content filters |
//! | [`GuardrailsConfig`] | Builder-pattern configuration |
//! | [`KeywordFilter`] | Simple keyword-based filtering |
//! | [`PatternFilter`] | Regex-based pattern matching |
//! | [`PiiDetector`] | Detection of personally identifiable information |
//! | [`CheckResult`] | Result of content checking with violations |
//!
//! ## Example
//!
//! ```rust
//! use ai_lib_rust::guardrails::{Guardrails, GuardrailsConfig, FilterAction};
//!
//! let config = GuardrailsConfig::builder()
//!     .add_keyword_filter("password", FilterAction::Block)
//!     .add_keyword_filter("secret", FilterAction::Warn)
//!     .enable_pii_detection(true)
//!     .build();
//!
//! let guardrails = Guardrails::new(config);
//!
//! // Check input before sending to AI
//! let result = guardrails.check_input("My password is 12345");
//! if result.is_blocked() {
//!     println!("Content blocked: {:?}", result.violations());
//! }
//!
//! // Sanitize content by redacting sensitive information
//! let sanitized = guardrails.sanitize("Email: user@example.com");
//! ```
//!
//! ## Presets
//!
//! - [`Guardrails::permissive()`] - Allow all content (for development)
//! - [`Guardrails::strict()`] - Strict safety defaults for production

mod config;
mod filters;
mod pii;
mod result;

pub use config::{FilterAction, FilterRule, GuardrailsConfig, GuardrailsConfigBuilder};
pub use filters::{KeywordFilter, PatternFilter, ContentFilter};
pub use pii::PiiDetector;
pub use result::{CheckResult, Violation, ViolationType};

use crate::types::message::Message;

/// Main guardrails controller for content filtering
#[derive(Debug, Clone)]
pub struct Guardrails {
    config: GuardrailsConfig,
    keyword_filter: KeywordFilter,
    pattern_filter: PatternFilter,
    pii_detector: Option<PiiDetector>,
}

impl Guardrails {
    /// Create a new Guardrails instance with the given configuration
    pub fn new(config: GuardrailsConfig) -> Self {
        let keyword_filter = KeywordFilter::from_rules(&config.keyword_rules);
        let pattern_filter = PatternFilter::from_rules(&config.pattern_rules);
        let pii_detector = if config.enable_pii_detection {
            Some(PiiDetector::new())
        } else {
            None
        };

        Self {
            config,
            keyword_filter,
            pattern_filter,
            pii_detector,
        }
    }

    /// Create a Guardrails instance with default (permissive) configuration
    pub fn permissive() -> Self {
        Self::new(GuardrailsConfig::permissive())
    }

    /// Create a Guardrails instance with strict safety defaults
    pub fn strict() -> Self {
        Self::new(GuardrailsConfig::strict())
    }

    /// Check input content before sending to the model
    pub fn check_input(&self, content: &str) -> CheckResult {
        self.check_content(content, true)
    }

    /// Check output content received from the model
    pub fn check_output(&self, content: &str) -> CheckResult {
        self.check_content(content, false)
    }

    /// Check a message (extracts text content and checks it)
    pub fn check_message(&self, message: &Message) -> CheckResult {
        let content = extract_text_content(message);
        self.check_content(&content, true)
    }

    /// Check multiple messages
    pub fn check_messages(&self, messages: &[Message]) -> CheckResult {
        let mut combined_result = CheckResult::passed();
        
        for message in messages {
            let result = self.check_message(message);
            combined_result = combined_result.merge(result);
            
            // Early exit if blocked and config says to stop on first block
            if combined_result.is_blocked() && self.config.stop_on_first_block {
                break;
            }
        }
        
        combined_result
    }

    /// Internal content checking logic
    fn check_content(&self, content: &str, is_input: bool) -> CheckResult {
        let mut violations = Vec::new();

        // Apply keyword filters
        if (is_input && self.config.filter_input) || (!is_input && self.config.filter_output) {
            violations.extend(self.keyword_filter.check(content));
            violations.extend(self.pattern_filter.check(content));
        }

        // Apply PII detection
        if let Some(ref pii_detector) = self.pii_detector {
            if (is_input && self.config.check_pii_input) || (!is_input && self.config.check_pii_output) {
                violations.extend(pii_detector.check(content));
            }
        }

        CheckResult::from_violations(violations)
    }

    /// Sanitize content by removing or replacing detected violations
    pub fn sanitize(&self, content: &str) -> String {
        let mut sanitized = content.to_string();

        // Sanitize keywords
        sanitized = self.keyword_filter.sanitize(&sanitized, &self.config.sanitize_replacement);

        // Sanitize patterns
        sanitized = self.pattern_filter.sanitize(&sanitized, &self.config.sanitize_replacement);

        // Sanitize PII
        if let Some(ref pii_detector) = self.pii_detector {
            sanitized = pii_detector.sanitize(&sanitized, &self.config.pii_replacement);
        }

        sanitized
    }

    /// Get the current configuration
    pub fn config(&self) -> &GuardrailsConfig {
        &self.config
    }
}

/// Extract text content from a Message
fn extract_text_content(message: &Message) -> String {
    use crate::types::message::MessageContent;
    
    match &message.content {
        MessageContent::Text(text) => text.clone(),
        MessageContent::Blocks(blocks) => {
            blocks
                .iter()
                .filter_map(|block| {
                    use crate::types::message::ContentBlock;
                    match block {
                        ContentBlock::Text { text } => Some(text.clone()),
                        _ => None,
                    }
                })
                .collect::<Vec<_>>()
                .join(" ")
        }
    }
}

impl Default for Guardrails {
    fn default() -> Self {
        Self::permissive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permissive_allows_all() {
        let guardrails = Guardrails::permissive();
        let result = guardrails.check_input("Any content should pass");
        assert!(result.is_passed());
    }

    #[test]
    fn test_keyword_blocking() {
        let config = GuardrailsConfig::builder()
            .add_keyword_filter("blocked_word", FilterAction::Block)
            .build();
        let guardrails = Guardrails::new(config);
        
        let result = guardrails.check_input("This contains blocked_word in it");
        assert!(result.is_blocked());
    }

    #[test]
    fn test_keyword_warning() {
        let config = GuardrailsConfig::builder()
            .add_keyword_filter("warn_word", FilterAction::Warn)
            .build();
        let guardrails = Guardrails::new(config);
        
        let result = guardrails.check_input("This contains warn_word in it");
        assert!(result.is_warned());
        assert!(!result.is_blocked());
    }

    #[test]
    fn test_sanitization() {
        let config = GuardrailsConfig::builder()
            .add_keyword_filter("secret", FilterAction::Sanitize)
            .sanitize_replacement("[REDACTED]".to_string())
            .build();
        let guardrails = Guardrails::new(config);
        
        let sanitized = guardrails.sanitize("My secret is here");
        assert!(sanitized.contains("[REDACTED]"));
        assert!(!sanitized.contains("secret"));
    }

    #[test]
    fn test_pii_detection() {
        let config = GuardrailsConfig::builder()
            .enable_pii_detection(true)
            .build();
        let guardrails = Guardrails::new(config);
        
        let result = guardrails.check_input("My email is test@example.com");
        assert!(result.has_violations());
    }
}
