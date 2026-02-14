//! Structured output module for ai-lib-rust.
//!
//! Provides JSON mode, schema validation, and structured output capabilities:
//! - `OutputValidator`: Validate JSON against schemas
//! - `ValidationResult`: Result of validation operations
//! - `ValidationError`: Detailed validation errors
//!
//! # Examples
//!
//! ```
//! use ai_lib_rust::structured::{OutputValidator, ValidationResult};
//! use serde_json::json;
//!
//! let schema = json!({
//!     "type": "object",
//!     "properties": {
//!         "name": {"type": "string"},
//!         "age": {"type": "integer"}
//!     },
//!     "required": ["name"]
//! });
//!
//! let validator = OutputValidator::lenient(schema);
//! let data = json!({"name": "Alice", "age": 30});
//! let result = validator.validate(data);
//!
//! assert!(result.is_valid());
//! ```

pub mod error;
pub mod json_mode;
pub mod schema;
pub mod validator;

// Re-export commonly used types
pub use error::{ValidationError, ValidationResult};
pub use json_mode::{JsonMode, JsonModeConfig, StructuredOutput};
pub use schema::{json_schema_from_type, schema_from_type_name, SchemaGenerator};
pub use validator::{IntoValidatorData, OutputValidator};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_mode_display() {
        assert_eq!(JsonMode::Json.to_string(), "json_object");
        assert_eq!(JsonMode::JsonSchema.to_string(), "json_schema");
        assert_eq!(JsonMode::Off.to_string(), "");
    }

    #[test]
    fn test_json_mode_from_str() {
        assert_eq!("json_object".parse::<JsonMode>().unwrap(), JsonMode::Json);
        assert_eq!(
            "json_schema".parse::<JsonMode>().unwrap(),
            JsonMode::JsonSchema
        );
        assert_eq!("off".parse::<JsonMode>().unwrap(), JsonMode::Off);
        assert_eq!("".parse::<JsonMode>().unwrap(), JsonMode::Off);

        assert!("invalid".parse::<JsonMode>().is_err());
    }
}
