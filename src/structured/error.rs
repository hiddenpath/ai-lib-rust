//! Error types for structured output validation.

use std::fmt;

/// Validation error with location information.
///
/// Contains details about what failed and where in the data structure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// Error message describing what went wrong
    pub message: String,
    /// JSON path to the error location (e.g., "user.name", "items[0].price")
    pub path: Option<String>,
    /// The invalid value that caused the error
    pub value: Option<serde_json::Value>,
}

impl ValidationError {
    /// Create a new validation error.
    ///
    /// # Arguments
    ///
    /// * `message` - Error description
    /// * `path` - Optional JSON path to the error location
    /// * `value` - Optional invalid value
    pub fn new(
        message: impl Into<String>,
        path: Option<String>,
        value: Option<serde_json::Value>,
    ) -> Self {
        Self {
            message: message.into(),
            path,
            value,
        }
    }

    /// Create an error with a path.
    pub fn with_path(message: impl Into<String>, path: String) -> Self {
        Self {
            message: message.into(),
            path: Some(path),
            value: None,
        }
    }

    /// Create an error without path.
    pub fn without_path(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: None,
            value: None,
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.path {
            Some(path) => write!(f, "{}: {}", path, self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Result of validation operation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,
    /// List of validation errors (empty if valid)
    pub errors: Vec<ValidationError>,
    /// Validated/parsed data (None if invalid)
    pub data: Option<serde_json::Value>,
}

impl ValidationResult {
    /// Create a successful validation result.
    pub fn success(data: serde_json::Value) -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            data: Some(data),
        }
    }

    /// Create a failed validation result.
    pub fn failure(errors: Vec<ValidationError>) -> Self {
        Self {
            valid: false,
            errors,
            data: None,
        }
    }

    /// Create a failure from a single error.
    pub fn from_error(error: ValidationError) -> Self {
        Self {
            valid: false,
            errors: vec![error],
            data: None,
        }
    }

    /// Create a failure from error messages (without paths).
    pub fn from_messages(messages: Vec<String>) -> Self {
        Self {
            valid: false,
            errors: messages
                .into_iter()
                .map(ValidationError::without_path)
                .collect(),
            data: None,
        }
    }

    /// Check if validation passed.
    pub fn is_valid(&self) -> bool {
        self.valid
    }

    /// Get the validated data.
    ///
    /// Returns None if validation failed.
    pub fn data(&self) -> Option<&serde_json::Value> {
        self.data.as_ref()
    }

    /// Get errors as formatted strings.
    pub fn error_messages(&self) -> Vec<String> {
        self.errors.iter().map(|e| e.to_string()).collect()
    }

    /// Merge multiple validation results.
    ///
    /// Returns success only if all results are successful.
    pub fn merge(results: Vec<ValidationResult>) -> Self {
        let mut all_errors = Vec::new();
        let mut all_valid = true;
        let mut final_data = None;
        let any_results = !results.is_empty();

        for result in results {
            if !result.valid {
                all_valid = false;
                all_errors.extend(result.errors);
            } else if final_data.is_none() {
                final_data = result.data;
            }
        }

        Self {
            valid: all_valid,
            errors: all_errors,
            data: if all_valid && any_results {
                final_data
            } else {
                None
            },
        }
    }

    /// Convert to Result, merging all errors if invalid.
    pub fn into_result(self) -> Result<serde_json::Value, Vec<ValidationError>> {
        if self.valid {
            Ok(self.data.unwrap_or(serde_json::Value::Null))
        } else {
            Err(self.errors)
        }
    }
}

impl From<ValidationError> for ValidationResult {
    fn from(error: ValidationError) -> Self {
        Self::from_error(error)
    }
}

impl From<Vec<ValidationError>> for ValidationResult {
    fn from(errors: Vec<ValidationError>) -> Self {
        Self::failure(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validation_error_display_without_path() {
        let error = ValidationError::without_path("Invalid type");
        assert_eq!(error.to_string(), "Invalid type");
        assert!(error.path.is_none());
    }

    #[test]
    fn test_validation_error_display_with_path() {
        let error = ValidationError::with_path("Invalid type", "user.name".to_string());
        assert_eq!(error.to_string(), "user.name: Invalid type");
        assert_eq!(error.path, Some("user.name".to_string()));
    }

    #[test]
    fn test_validation_result_success() {
        let data = serde_json::json!({"name": "Alice"});
        let result = ValidationResult::success(data.clone());

        assert!(result.is_valid());
        assert_eq!(result.data(), Some(&data));
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_validation_result_failure() {
        let errors = vec![
            ValidationError::with_path("Missing field", "user.name".to_string()),
            ValidationError::with_path("Invalid type", "user.age".to_string()),
        ];
        let result = ValidationResult::failure(errors.clone());

        assert!(!result.is_valid());
        assert!(result.data.is_none());
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn test_validation_result_from_error() {
        let error = ValidationError::without_path("Test error");
        let result = ValidationResult::from_error(error.clone());

        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 1);
        let first_error = &result.errors[0];
        assert!(first_error.path.is_none());
    }

    #[test]
    fn test_validation_result_from_messages() {
        let messages = vec!["Error 1".to_string(), "Error 2".to_string()];
        let result = ValidationResult::from_messages(messages);

        assert!(!result.is_valid());
        assert_eq!(result.errors.len(), 2);
        assert!(result.errors[0].path.is_none());
    }

    #[test]
    fn test_validation_result_merge_all_success() {
        let result1 = ValidationResult::success(serde_json::json!(1));
        let result2 = ValidationResult::success(serde_json::json!(2));

        let merged = ValidationResult::merge(vec![result1, result2]);
        assert!(merged.is_valid());
    }

    #[test]
    fn test_validation_result_merge_one_failure() {
        let result1 = ValidationResult::success(serde_json::json!(1));
        let error = ValidationError::without_path("Test error");
        let result2 = ValidationResult::from_error(error);

        let merged = ValidationResult::merge(vec![result1, result2]);
        assert!(!merged.is_valid());
        assert_eq!(merged.errors.len(), 1);
    }

    #[test]
    fn test_validation_result_into_result_success() {
        let data = serde_json::json!({"test": 123});
        let result = ValidationResult::success(data.clone());

        assert_eq!(result.into_result(), Ok(data));
    }

    #[test]
    fn test_validation_result_into_result_failure() {
        let errors = vec![ValidationError::without_path("Test error")];
        let result = ValidationResult::failure(errors.clone());

        assert_eq!(result.into_result(), Err(errors));
    }

    #[test]
    fn test_error_messages() {
        let errors = vec![
            ValidationError::with_path("Error 1", "path1".to_string()),
            ValidationError::without_path("Error 2"),
        ];
        let result = ValidationResult::failure(errors);

        let messages = result.error_messages();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], "path1: Error 1");
        assert_eq!(messages[1], "Error 2");
    }
}
