//! Check result types

use super::config::FilterAction;
use serde::{Deserialize, Serialize};

/// Type of violation detected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ViolationType {
    /// Keyword match
    Keyword,
    /// Regex pattern match
    Pattern,
    /// PII detection
    Pii,
    /// Custom rule
    Custom,
}

/// A detected violation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    /// Type of violation
    pub violation_type: ViolationType,
    /// The pattern or rule that matched
    pub pattern: String,
    /// Action associated with this violation
    pub action: FilterAction,
    /// Category of the violation
    pub category: Option<String>,
    /// Description of the violation
    pub description: Option<String>,
    /// The matched text (may be masked for sensitive data)
    pub matched_text: Option<String>,
}

impl Violation {
    /// Check if this violation should block the content
    pub fn is_blocking(&self) -> bool {
        matches!(self.action, FilterAction::Block)
    }

    /// Check if this violation is a warning
    pub fn is_warning(&self) -> bool {
        matches!(self.action, FilterAction::Warn)
    }
}

/// Result of a content check
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CheckResult {
    /// List of violations found
    violations: Vec<Violation>,
    /// Whether the content should be blocked
    blocked: bool,
    /// Whether warnings were generated
    warned: bool,
}

impl CheckResult {
    /// Create a passed result (no violations)
    pub fn passed() -> Self {
        Self {
            violations: Vec::new(),
            blocked: false,
            warned: false,
        }
    }

    /// Create a result from violations
    pub fn from_violations(violations: Vec<Violation>) -> Self {
        let blocked = violations.iter().any(|v| v.is_blocking());
        let warned = violations.iter().any(|v| v.is_warning());
        
        Self {
            violations,
            blocked,
            warned,
        }
    }

    /// Check if the content passed all checks
    pub fn is_passed(&self) -> bool {
        !self.blocked && self.violations.is_empty()
    }

    /// Check if the content was blocked
    pub fn is_blocked(&self) -> bool {
        self.blocked
    }

    /// Check if warnings were generated
    pub fn is_warned(&self) -> bool {
        self.warned
    }

    /// Check if there are any violations
    pub fn has_violations(&self) -> bool {
        !self.violations.is_empty()
    }

    /// Get the list of violations
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// Get blocking violations only
    pub fn blocking_violations(&self) -> Vec<&Violation> {
        self.violations.iter().filter(|v| v.is_blocking()).collect()
    }

    /// Get warning violations only
    pub fn warning_violations(&self) -> Vec<&Violation> {
        self.violations.iter().filter(|v| v.is_warning()).collect()
    }

    /// Merge another result into this one
    pub fn merge(mut self, other: CheckResult) -> Self {
        self.violations.extend(other.violations);
        self.blocked = self.blocked || other.blocked;
        self.warned = self.warned || other.warned;
        self
    }

    /// Get a summary string
    pub fn summary(&self) -> String {
        if self.is_passed() {
            "PASSED".to_string()
        } else if self.is_blocked() {
            format!("BLOCKED: {} violation(s)", self.violations.len())
        } else if self.is_warned() {
            format!("WARNING: {} violation(s)", self.violations.len())
        } else {
            format!("INFO: {} item(s) logged", self.violations.len())
        }
    }
}

impl std::fmt::Display for CheckResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.summary())
    }
}
