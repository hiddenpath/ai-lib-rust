//! PII (Personally Identifiable Information) detection

use super::config::FilterAction;
use super::result::{Violation, ViolationType};

/// PII detector for identifying personally identifiable information
#[derive(Debug, Clone)]
pub struct PiiDetector {
    /// Email detection pattern
    email_pattern: regex::Regex,
    /// Phone number patterns (US format)
    phone_pattern: regex::Regex,
    /// Credit card pattern (basic)
    credit_card_pattern: regex::Regex,
    /// SSN pattern (US format)
    ssn_pattern: regex::Regex,
    /// IP address pattern
    ip_pattern: regex::Regex,
}

impl PiiDetector {
    /// Create a new PII detector with default patterns
    pub fn new() -> Self {
        Self {
            email_pattern: regex::Regex::new(
                r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}"
            ).unwrap(),
            phone_pattern: regex::Regex::new(
                r"(?:\+?1[-.\s]?)?(?:\(?[0-9]{3}\)?[-.\s]?)?[0-9]{3}[-.\s]?[0-9]{4}"
            ).unwrap(),
            credit_card_pattern: regex::Regex::new(
                r"\b(?:4[0-9]{12}(?:[0-9]{3})?|5[1-5][0-9]{14}|3[47][0-9]{13}|6(?:011|5[0-9]{2})[0-9]{12})\b"
            ).unwrap(),
            ssn_pattern: regex::Regex::new(
                r"\b[0-9]{3}[-\s]?[0-9]{2}[-\s]?[0-9]{4}\b"
            ).unwrap(),
            ip_pattern: regex::Regex::new(
                r"\b(?:(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(?:25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\b"
            ).unwrap(),
        }
    }

    /// Check content for PII
    pub fn check(&self, content: &str) -> Vec<Violation> {
        let mut violations = Vec::new();

        // Check for emails
        for m in self.email_pattern.find_iter(content) {
            violations.push(Violation {
                violation_type: ViolationType::Pii,
                pattern: "email".to_string(),
                action: FilterAction::Warn,
                category: Some("pii".to_string()),
                description: Some("Email address detected".to_string()),
                matched_text: Some(m.as_str().to_string()),
            });
        }

        // Check for phone numbers
        for m in self.phone_pattern.find_iter(content) {
            // Filter out short matches that are likely not phone numbers
            if m.as_str().len() >= 10 {
                violations.push(Violation {
                    violation_type: ViolationType::Pii,
                    pattern: "phone".to_string(),
                    action: FilterAction::Warn,
                    category: Some("pii".to_string()),
                    description: Some("Phone number detected".to_string()),
                    matched_text: Some(m.as_str().to_string()),
                });
            }
        }

        // Check for credit card numbers
        for m in self.credit_card_pattern.find_iter(content) {
            if Self::is_valid_credit_card(m.as_str()) {
                violations.push(Violation {
                    violation_type: ViolationType::Pii,
                    pattern: "credit_card".to_string(),
                    action: FilterAction::Block,
                    category: Some("pii".to_string()),
                    description: Some("Credit card number detected".to_string()),
                    matched_text: Some(Self::mask_credit_card(m.as_str())),
                });
            }
        }

        // Check for SSN
        for _m in self.ssn_pattern.find_iter(content) {
            violations.push(Violation {
                violation_type: ViolationType::Pii,
                pattern: "ssn".to_string(),
                action: FilterAction::Block,
                category: Some("pii".to_string()),
                description: Some("Social Security Number detected".to_string()),
                matched_text: Some("XXX-XX-XXXX".to_string()),
            });
        }

        // Check for IP addresses (informational)
        for m in self.ip_pattern.find_iter(content) {
            violations.push(Violation {
                violation_type: ViolationType::Pii,
                pattern: "ip_address".to_string(),
                action: FilterAction::Log,
                category: Some("pii".to_string()),
                description: Some("IP address detected".to_string()),
                matched_text: Some(m.as_str().to_string()),
            });
        }

        violations
    }

    /// Sanitize PII from content
    pub fn sanitize(&self, content: &str, replacement: &str) -> String {
        let mut result = content.to_string();

        // Replace emails
        result = self.email_pattern.replace_all(&result, replacement).to_string();

        // Replace phone numbers (only longer matches)
        result = self.phone_pattern.replace_all(&result, |caps: &regex::Captures| {
            if caps[0].len() >= 10 {
                replacement.to_string()
            } else {
                caps[0].to_string()
            }
        }).to_string();

        // Replace credit cards
        result = self.credit_card_pattern.replace_all(&result, replacement).to_string();

        // Replace SSN
        result = self.ssn_pattern.replace_all(&result, replacement).to_string();

        result
    }

    /// Basic Luhn algorithm check for credit card validation
    fn is_valid_credit_card(number: &str) -> bool {
        let digits: Vec<u32> = number
            .chars()
            .filter(|c| c.is_ascii_digit())
            .filter_map(|c| c.to_digit(10))
            .collect();

        if digits.len() < 13 || digits.len() > 19 {
            return false;
        }

        let mut sum = 0;
        let mut double = false;

        for digit in digits.iter().rev() {
            let mut d = *digit;
            if double {
                d *= 2;
                if d > 9 {
                    d -= 9;
                }
            }
            sum += d;
            double = !double;
        }

        sum % 10 == 0
    }

    /// Mask credit card number for logging
    fn mask_credit_card(number: &str) -> String {
        let digits: String = number.chars().filter(|c| c.is_ascii_digit()).collect();
        if digits.len() >= 4 {
            format!("****-****-****-{}", &digits[digits.len()-4..])
        } else {
            "****".to_string()
        }
    }
}

impl Default for PiiDetector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_detection() {
        let detector = PiiDetector::new();
        let violations = detector.check("Contact me at test@example.com");
        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.pattern == "email"));
    }

    #[test]
    fn test_phone_detection() {
        let detector = PiiDetector::new();
        let violations = detector.check("Call me at 555-123-4567");
        assert!(violations.iter().any(|v| v.pattern == "phone"));
    }

    #[test]
    fn test_credit_card_detection() {
        let detector = PiiDetector::new();
        // Valid test card number (Visa)
        let violations = detector.check("Card: 4111111111111111");
        assert!(violations.iter().any(|v| v.pattern == "credit_card"));
    }

    #[test]
    fn test_ssn_detection() {
        let detector = PiiDetector::new();
        let violations = detector.check("SSN: 123-45-6789");
        assert!(violations.iter().any(|v| v.pattern == "ssn"));
    }

    #[test]
    fn test_sanitization() {
        let detector = PiiDetector::new();
        let sanitized = detector.sanitize("Email: test@example.com", "[REDACTED]");
        assert!(!sanitized.contains("@"));
        assert!(sanitized.contains("[REDACTED]"));
    }
}
