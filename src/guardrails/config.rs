//! Guardrails configuration

use serde::{Deserialize, Serialize};

/// Action to take when a filter rule matches
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterAction {
    /// Block the content entirely
    Block,
    /// Allow but log a warning
    Warn,
    /// Log for audit purposes only
    Log,
    /// Sanitize (remove/replace) the matched content
    Sanitize,
    /// Allow without any action
    Allow,
}

impl Default for FilterAction {
    fn default() -> Self {
        FilterAction::Warn
    }
}

/// A filter rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterRule {
    /// The pattern or keyword to match
    pub pattern: String,
    /// Whether this is a regex pattern
    pub is_regex: bool,
    /// Case-sensitive matching
    pub case_sensitive: bool,
    /// Action to take when matched
    pub action: FilterAction,
    /// Optional category for grouping
    pub category: Option<String>,
    /// Optional description
    pub description: Option<String>,
}

impl FilterRule {
    /// Create a simple keyword rule
    pub fn keyword(pattern: impl Into<String>, action: FilterAction) -> Self {
        Self {
            pattern: pattern.into(),
            is_regex: false,
            case_sensitive: false,
            action,
            category: None,
            description: None,
        }
    }

    /// Create a regex pattern rule
    pub fn regex(pattern: impl Into<String>, action: FilterAction) -> Self {
        Self {
            pattern: pattern.into(),
            is_regex: true,
            case_sensitive: true,
            action,
            category: None,
            description: None,
        }
    }

    /// Set case sensitivity
    pub fn case_sensitive(mut self, sensitive: bool) -> Self {
        self.case_sensitive = sensitive;
        self
    }

    /// Set category
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }
}

/// Configuration for the Guardrails system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardrailsConfig {
    /// Whether to filter input content
    pub filter_input: bool,
    /// Whether to filter output content
    pub filter_output: bool,
    /// Keyword-based filter rules
    pub keyword_rules: Vec<FilterRule>,
    /// Regex pattern-based filter rules
    pub pattern_rules: Vec<FilterRule>,
    /// Enable PII (Personally Identifiable Information) detection
    pub enable_pii_detection: bool,
    /// Check PII in input
    pub check_pii_input: bool,
    /// Check PII in output
    pub check_pii_output: bool,
    /// Replacement string for sanitization
    pub sanitize_replacement: String,
    /// Replacement string for PII
    pub pii_replacement: String,
    /// Stop checking on first block
    pub stop_on_first_block: bool,
}

impl GuardrailsConfig {
    /// Create a builder for GuardrailsConfig
    pub fn builder() -> GuardrailsConfigBuilder {
        GuardrailsConfigBuilder::default()
    }

    /// Create a permissive configuration (no filtering)
    pub fn permissive() -> Self {
        Self {
            filter_input: false,
            filter_output: false,
            keyword_rules: Vec::new(),
            pattern_rules: Vec::new(),
            enable_pii_detection: false,
            check_pii_input: false,
            check_pii_output: false,
            sanitize_replacement: "[FILTERED]".to_string(),
            pii_replacement: "[PII]".to_string(),
            stop_on_first_block: false,
        }
    }

    /// Create a strict configuration with common safety rules
    pub fn strict() -> Self {
        let mut config = Self::permissive();
        config.filter_input = true;
        config.filter_output = true;
        config.enable_pii_detection = true;
        config.check_pii_input = true;
        config.check_pii_output = true;
        config.stop_on_first_block = true;
        
        // Add common sensitive keyword patterns
        config.keyword_rules = vec![
            FilterRule::keyword("password", FilterAction::Warn)
                .with_category("credentials")
                .with_description("Password mention"),
            FilterRule::keyword("api_key", FilterAction::Warn)
                .with_category("credentials")
                .with_description("API key mention"),
            FilterRule::keyword("secret_key", FilterAction::Warn)
                .with_category("credentials")
                .with_description("Secret key mention"),
            FilterRule::keyword("access_token", FilterAction::Warn)
                .with_category("credentials")
                .with_description("Access token mention"),
        ];

        config
    }
}

impl Default for GuardrailsConfig {
    fn default() -> Self {
        Self::permissive()
    }
}

/// Builder for GuardrailsConfig
#[derive(Debug, Default)]
pub struct GuardrailsConfigBuilder {
    filter_input: bool,
    filter_output: bool,
    keyword_rules: Vec<FilterRule>,
    pattern_rules: Vec<FilterRule>,
    enable_pii_detection: bool,
    check_pii_input: bool,
    check_pii_output: bool,
    sanitize_replacement: Option<String>,
    pii_replacement: Option<String>,
    stop_on_first_block: bool,
}

impl GuardrailsConfigBuilder {
    /// Enable input filtering
    pub fn filter_input(mut self, enable: bool) -> Self {
        self.filter_input = enable;
        self
    }

    /// Enable output filtering
    pub fn filter_output(mut self, enable: bool) -> Self {
        self.filter_output = enable;
        self
    }

    /// Add a keyword filter rule
    pub fn add_keyword_filter(mut self, keyword: impl Into<String>, action: FilterAction) -> Self {
        self.filter_input = true; // Auto-enable input filtering
        self.keyword_rules.push(FilterRule::keyword(keyword, action));
        self
    }

    /// Add a regex pattern filter rule
    pub fn add_pattern_filter(mut self, pattern: impl Into<String>, action: FilterAction) -> Self {
        self.filter_input = true; // Auto-enable input filtering
        self.pattern_rules.push(FilterRule::regex(pattern, action));
        self
    }

    /// Add a custom filter rule
    pub fn add_rule(mut self, rule: FilterRule) -> Self {
        self.filter_input = true;
        if rule.is_regex {
            self.pattern_rules.push(rule);
        } else {
            self.keyword_rules.push(rule);
        }
        self
    }

    /// Enable PII detection
    pub fn enable_pii_detection(mut self, enable: bool) -> Self {
        self.enable_pii_detection = enable;
        self.check_pii_input = enable;
        self.check_pii_output = enable;
        self
    }

    /// Set the sanitize replacement string
    pub fn sanitize_replacement(mut self, replacement: String) -> Self {
        self.sanitize_replacement = Some(replacement);
        self
    }

    /// Set the PII replacement string
    pub fn pii_replacement(mut self, replacement: String) -> Self {
        self.pii_replacement = Some(replacement);
        self
    }

    /// Stop on first block
    pub fn stop_on_first_block(mut self, stop: bool) -> Self {
        self.stop_on_first_block = stop;
        self
    }

    /// Build the configuration
    pub fn build(self) -> GuardrailsConfig {
        GuardrailsConfig {
            filter_input: self.filter_input,
            filter_output: self.filter_output,
            keyword_rules: self.keyword_rules,
            pattern_rules: self.pattern_rules,
            enable_pii_detection: self.enable_pii_detection,
            check_pii_input: self.check_pii_input,
            check_pii_output: self.check_pii_output,
            sanitize_replacement: self.sanitize_replacement.unwrap_or_else(|| "[FILTERED]".to_string()),
            pii_replacement: self.pii_replacement.unwrap_or_else(|| "[PII]".to_string()),
            stop_on_first_block: self.stop_on_first_block,
        }
    }
}
