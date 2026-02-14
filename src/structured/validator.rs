//! Output validator for structured responses.
//!
//! Validates JSON data against JSON schemas, supporting:
//! - Basic type validation (string, integer, number, boolean, array, object, null)
//! - Field constraints (minLength, maxLength, minimum, maximum, pattern, enum)
//! - Array constraints (minItems, maxItems, items schema)
//! - Nested validation (recursive object and array validation)
//! - Additional properties control

use crate::structured::error::{ValidationError, ValidationResult};
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;

/// Validator for structured output.
///
/// Validates JSON data against JSON schemas with full error reporting.
pub struct OutputValidator {
    /// The JSON schema to validate against
    schema: Option<Value>,
    /// Whether to use strict validation mode
    strict: bool,
}

impl OutputValidator {
    /// Create a new validator with a schema.
    ///
    /// # Arguments
    ///
    /// * `schema` - JSON schema as a serde_json::Value
    /// * `strict` - Whether to use strict validation (disallow extra properties by default)
    pub fn new(schema: Value, strict: bool) -> Self {
        Self {
            schema: Some(schema),
            strict,
        }
    }

    /// Create a new validator with a schema (strict mode enabled).
    pub fn strict(schema: Value) -> Self {
        Self::new(schema, true)
    }

    /// Create a new validator with a schema (strict mode disabled).
    pub fn lenient(schema: Value) -> Self {
        Self::new(schema, false)
    }

    /// Create a validator without a schema (permissive mode).
    pub fn permissive() -> Self {
        Self {
            schema: None,
            strict: false,
        }
    }

    /// Validate data against the schema.
    ///
    /// # Arguments
    ///
    /// * `data` - Data to validate (can be JSON string, JSON value, or arbitrary Rust value)
    ///
    /// # Returns
    ///
    /// A ValidationResult with validation status and any errors.
    pub fn validate(&self, data: impl IntoValidatorData) -> ValidationResult {
        let parsed = data.into_value();

        // If no schema is configured, always succeed
        let schema = match &self.schema {
            Some(s) => s.clone(),
            None => return ValidationResult::success(parsed),
        };

        self.validate_against_schema(&parsed, &schema, "")
    }

    /// Validate data and return the validated value or merge errors.
    ///
    /// # Arguments
    ///
    /// * `data` - Data to validate
    ///
    /// # Returns
    ///
    /// Ok(validated_value) if validation succeeds, Err(errors) if it fails.
    pub fn validate_or_fail(
        &self,
        data: impl IntoValidatorData,
    ) -> Result<Value, Vec<ValidationError>> {
        self.validate(data).into_result()
    }

    /// Validate data against a schema at a specific path.
    fn validate_against_schema(
        &self,
        data: &Value,
        schema: &Value,
        path: &str,
    ) -> ValidationResult {
        let mut errors = Vec::new();

        // Type validation
        let schema_type = schema.get("type").and_then(|t| t.as_str());
        if let Some(type_name) = schema_type {
            if let Err(e) = self.validate_type(data, type_name, path) {
                errors.push(e);
                return ValidationResult::failure(errors);
            }
        }

        // Null handling (nullable)
        let is_nullable = schema
            .get("nullable")
            .and_then(|n| n.as_bool())
            .unwrap_or(false);
        if is_nullable && data.is_null() {
            return ValidationResult::success(data.clone());
        }

        // String-specific validation
        if schema_type == Some("string") && data.is_string() {
            self.validate_string(data, schema, path, &mut errors);
        }

        // Number-specific validation
        if matches!(schema_type, Some("integer") | Some("number")) {
            if let Some(num) = data.as_f64() {
                self.validate_number(num, schema, path, &mut errors);
            }
        }

        // Array validation
        if schema_type == Some("array") && data.is_array() {
            self.validate_array(data, schema, path, &mut errors);
        }

        // Object validation
        if schema_type == Some("object") && data.is_object() {
            self.validate_object(data, schema, path, &mut errors);
        }

        // Enum validation
        if let Some(enum_values) = schema.get("enum").and_then(|e| e.as_array()) {
            self.validate_enum(data, enum_values, path, &mut errors);
        }

        if errors.is_empty() {
            ValidationResult::success(data.clone())
        } else {
            ValidationResult::failure(errors)
        }
    }

    /// Validate the type of a value.
    fn validate_type(
        &self,
        data: &Value,
        expected_type: &str,
        path: &str,
    ) -> Result<(), ValidationError> {
        let is_valid = match expected_type {
            "string" => data.is_string(),
            "integer" => data.is_i64(),
            "number" => data.is_number(),
            "boolean" => data.is_boolean(),
            "array" => data.is_array(),
            "object" => data.is_object(),
            "null" => data.is_null(),
            _ => true, // Unknown type, accept anything
        };

        if !is_valid {
            let actual_type = match data {
                Value::String(_) => "string",
                Value::Number(_) => {
                    if data.as_i64().is_some() {
                        "integer"
                    } else {
                        "number"
                    }
                }
                Value::Bool(_) => "boolean",
                Value::Array(_) => "array",
                Value::Object(_) => "object",
                Value::Null => "null",
            };
            Err(ValidationError::with_path(
                format!("Expected type '{}', got '{}'", expected_type, actual_type),
                path.to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Validate string constraints.
    fn validate_string(
        &self,
        data: &Value,
        schema: &Value,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        let s = match data.as_str() {
            Some(s) => s,
            None => return,
        };

        // minLength
        if let Some(min_length) = schema.get("minLength").and_then(|m| m.as_u64()) {
            if s.len() < min_length as usize {
                errors.push(ValidationError::with_path(
                    format!("String too short (minimum {} characters)", min_length),
                    path.to_string(),
                ));
            }
        }

        // maxLength
        if let Some(max_length) = schema.get("maxLength").and_then(|m| m.as_u64()) {
            if s.len() > max_length as usize {
                errors.push(ValidationError::with_path(
                    format!("String too long (maximum {} characters)", max_length),
                    path.to_string(),
                ));
            }
        }

        // pattern (regex)
        if let Some(pattern) = schema.get("pattern").and_then(|p| p.as_str()) {
            match Regex::new(pattern) {
                Ok(re) => {
                    if !re.is_match(s) {
                        errors.push(ValidationError::with_path(
                            "String does not match required pattern".to_string(),
                            path.to_string(),
                        ));
                    }
                }
                Err(_) => {
                    // Invalid regex, skip validation
                }
            }
        }
    }

    /// Validate number constraints.
    fn validate_number(
        &self,
        value: f64,
        schema: &Value,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        // minimum
        if let Some(minimum) = schema.get("minimum").and_then(|m| m.as_f64()) {
            if value < minimum {
                errors.push(ValidationError::with_path(
                    format!("Value below minimum ({})", minimum),
                    path.to_string(),
                ));
            }
        }

        // maximum
        if let Some(maximum) = schema.get("maximum").and_then(|m| m.as_f64()) {
            if value > maximum {
                errors.push(ValidationError::with_path(
                    format!("Value above maximum ({})", maximum),
                    path.to_string(),
                ));
            }
        }
    }

    /// Validate array constraints.
    fn validate_array(
        &self,
        data: &Value,
        schema: &Value,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        let arr = match data.as_array() {
            Some(a) => a,
            None => return,
        };

        // minItems
        if let Some(min_items) = schema.get("minItems").and_then(|m| m.as_u64()) {
            if arr.len() < min_items as usize {
                errors.push(ValidationError::with_path(
                    format!("Array too short (minimum {} items)", min_items),
                    path.to_string(),
                ));
            }
        }

        // maxItems
        if let Some(max_items) = schema.get("maxItems").and_then(|m| m.as_u64()) {
            if arr.len() > max_items as usize {
                errors.push(ValidationError::with_path(
                    format!("Array too long (maximum {} items)", max_items),
                    path.to_string(),
                ));
            }
        }

        // items (validate each element)
        if let Some(items_schema) = schema.get("items") {
            for (i, item) in arr.iter().enumerate() {
                let item_path = format!("{}[{}]", path, i);
                let result = self.validate_against_schema(item, items_schema, &item_path);
                if !result.is_valid() {
                    errors.extend(result.errors);
                }
            }
        }
    }

    /// Validate object constraints.
    fn validate_object(
        &self,
        data: &Value,
        schema: &Value,
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        let obj = match data.as_object() {
            Some(o) => o,
            None => return,
        };

        // required properties
        let required: Vec<String> = schema
            .get("required")
            .and_then(|r| r.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        for prop_name in &required {
            if !obj.contains_key(prop_name) {
                errors.push(ValidationError::with_path(
                    format!("Missing required property: {}", prop_name),
                    format!("{}.{}", path, prop_name),
                ));
            }
        }

        // properties (validate each property)
        let empty_props: Value = serde_json::json!({});
        let properties = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap_or_else(|| empty_props.as_object().unwrap());

        for (prop_name, prop_schema) in properties {
            if let Some(prop_value) = obj.get(prop_name) {
                let prop_path = format!("{}.{}", path, prop_name);
                let result = self.validate_against_schema(prop_value, prop_schema, &prop_path);
                if !result.is_valid() {
                    errors.extend(result.errors);
                }
            }
        }

        // additionalProperties
        let additional_props = schema
            .get("additionalProperties")
            .and_then(|a| a.as_bool())
            .unwrap_or(!self.strict); // Default to opposite of strict mode

        if !additional_props {
            let allowed_keys: HashSet<&str> = properties.keys().map(|k| k.as_str()).collect();
            for key in obj.keys() {
                if !allowed_keys.contains(key.as_str()) {
                    errors.push(ValidationError::with_path(
                        format!("Additional property not allowed: {}", key),
                        format!("{}.{}", path, key),
                    ));
                }
            }
        }

        // additionalProperties as schema
        if let Some(additional_schema) =
            schema.get("additionalProperties").and_then(
                |a| {
                    if a.is_boolean() {
                        None
                    } else {
                        Some(a)
                    }
                },
            )
        {
            let allowed_keys: HashSet<&str> = properties.keys().map(|k| k.as_str()).collect();
            for (key, value) in obj {
                if !allowed_keys.contains(key.as_str()) {
                    let prop_path = format!("{}.{}", path, key);
                    let result = self.validate_against_schema(value, additional_schema, &prop_path);
                    if !result.is_valid() {
                        errors.extend(result.errors);
                    }
                }
            }
        }
    }

    /// Validate enum constraint.
    fn validate_enum(
        &self,
        data: &Value,
        enum_values: &[Value],
        path: &str,
        errors: &mut Vec<ValidationError>,
    ) {
        if !enum_values.contains(data) {
            let allowed: Vec<String> = enum_values
                .iter()
                .map(|v| match v {
                    Value::String(s) => format!("\"{}\"", s),
                    _ => v.to_string(),
                })
                .collect();
            errors.push(ValidationError::with_path(
                format!("Value not in allowed enum values: {}", allowed.join(", ")),
                path.to_string(),
            ));
        }
    }
}

/// Trait for types that can be converted to validator data.
pub trait IntoValidatorData {
    fn into_value(self) -> Value;
}

impl IntoValidatorData for Value {
    fn into_value(self) -> Value {
        self
    }
}

impl IntoValidatorData for &Value {
    fn into_value(self) -> Value {
        self.clone()
    }
}

impl IntoValidatorData for &str {
    fn into_value(self) -> Value {
        // Try to parse as JSON, fall back to string
        serde_json::from_str(self).unwrap_or_else(|_| Value::String(self.to_string()))
    }
}

impl IntoValidatorData for String {
    fn into_value(self) -> Value {
        // Try to parse as JSON, fall back to string
        #[allow(clippy::unnecessary_lazy_evaluations)]
        serde_json::from_str(&self).unwrap_or_else(|_| Value::String(self))
    }
}

impl IntoValidatorData for i64 {
    fn into_value(self) -> Value {
        Value::Number(self.into())
    }
}

impl IntoValidatorData for i32 {
    fn into_value(self) -> Value {
        Value::Number(self.into())
    }
}

impl IntoValidatorData for u64 {
    fn into_value(self) -> Value {
        Value::Number(self.into())
    }
}

impl IntoValidatorData for u32 {
    fn into_value(self) -> Value {
        Value::Number(self.into())
    }
}

impl IntoValidatorData for f64 {
    fn into_value(self) -> Value {
        serde_json::Number::from_f64(self)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

impl IntoValidatorData for f32 {
    fn into_value(self) -> Value {
        serde_json::Number::from_f64(self as f64)
            .map(Value::Number)
            .unwrap_or(Value::Null)
    }
}

impl IntoValidatorData for bool {
    fn into_value(self) -> Value {
        Value::Bool(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_string_schema() -> Value {
        serde_json::json!({
            "type": "string"
        })
    }

    fn make_string_schema_with_length(min: Option<u64>, max: Option<u64>) -> Value {
        let mut schema = serde_json::json!({
            "type": "string"
        });
        if let Some(m) = min {
            schema["minLength"] = m.into();
        }
        if let Some(m) = max {
            schema["maxLength"] = m.into();
        }
        schema
    }

    fn make_object_schema(required: Vec<String>) -> Value {
        let mut schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"},
                "age": {"type": "integer"}
            }
        });
        if !required.is_empty() {
            schema["required"] = serde_json::json!(required);
        }
        schema
    }

    fn make_array_schema() -> Value {
        serde_json::json!({
            "type": "array",
            "items": {"type": "string"}
        })
    }

    #[test]
    fn test_validator_basic_string() {
        let validator = OutputValidator::lenient(make_string_schema());

        let result = validator.validate("hello");
        assert!(result.is_valid());
    }

    #[test]
    fn test_validator_string_min_length() {
        let validator = OutputValidator::lenient(make_string_schema_with_length(Some(5), None));

        let result = validator.validate("hi");
        assert!(!result.is_valid());
        assert!(result.error_messages()[0].contains("too short"));
    }

    #[test]
    fn test_validator_string_max_length() {
        let validator = OutputValidator::lenient(make_string_schema_with_length(None, Some(3)));

        let result = validator.validate("hello");
        assert!(!result.is_valid());
        assert!(result.error_messages()[0].contains("too long"));
    }

    #[test]
    fn test_validator_integer_type() {
        let schema = serde_json::json!({"type": "integer"});
        let validator = OutputValidator::lenient(schema);

        let result = validator.validate(42_i32);
        assert!(result.is_valid());

        let result = validator.validate(serde_json::Value::String("42".to_string()));
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validator_object_required() {
        let schema = make_object_schema(vec!["name".to_string()]);
        let validator = OutputValidator::lenient(schema);

        let data = serde_json::json!({"age": 30});
        let result = validator.validate(data);
        assert!(!result.is_valid());
        assert!(result.error_messages()[0].contains("Missing required"));
    }

    #[test]
    fn test_validator_array_items() {
        let validator = OutputValidator::lenient(make_array_schema());

        let data = serde_json::json!(["hello", "world"]);
        let result = validator.validate(data);
        assert!(result.is_valid());

        let data = serde_json::json!([1, 2, 3]);
        let result = validator.validate(data);
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validator_enum() {
        let schema = serde_json::json!({
            "type": "string",
            "enum": ["red", "green", "blue"]
        });
        let validator = OutputValidator::lenient(schema);

        let result = validator.validate("red");
        assert!(result.is_valid());

        let result = validator.validate("yellow");
        assert!(!result.is_valid());
        assert!(result.error_messages()[0].contains("not in allowed enum"));
    }

    #[test]
    fn test_validator_permissive() {
        let validator = OutputValidator::permissive();

        let result = validator.validate(serde_json::json!({"arbitrary": "data"}));
        assert!(result.is_valid());
    }

    #[test]
    fn test_validator_strict_additional_properties() {
        let schema = make_object_schema(vec![]);
        let validator = OutputValidator::strict(schema);

        let data = serde_json::json!({"name": "Alice", "extra": "data"});
        let result = validator.validate(data);
        assert!(!result.is_valid());
        assert!(result.error_messages()[0].contains("Additional property not allowed"));
    }

    #[test]
    fn test_validator_nested_object() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "user": {
                    "type": "object",
                    "properties": {
                        "name": {"type": "string"}
                    },
                    "required": ["name"]
                }
            }
        });
        let validator = OutputValidator::lenient(schema);

        let data = serde_json::json!({"user": {"age": 30}});
        let result = validator.validate(data);
        assert!(!result.is_valid());
        // Check that error message contains "required" instead of "missing"
        assert!(result.error_messages()[0]
            .to_lowercase()
            .contains("required"));
    }

    #[test]
    fn test_validate_or_fail() {
        let validator = OutputValidator::lenient(make_string_schema());

        let result = validator.validate_or_fail("hello");
        assert!(result.is_ok());

        let schema = serde_json::json!({"type": "integer"});
        let validator = OutputValidator::lenient(schema);
        let result = validator.validate_or_fail("hello");
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_result_merge() {
        let result1 = ValidationResult::success(serde_json::json!(1));
        let result2 = ValidationResult::success(serde_json::json!(2));
        let merged = ValidationResult::merge(vec![result1, result2]);
        assert!(merged.is_valid());

        let error = ValidationError::without_path("Test error");
        let result3 = ValidationResult::from_error(error);
        let merged = ValidationResult::merge(vec![
            ValidationResult::success(serde_json::json!(1)),
            result3,
        ]);
        assert!(!merged.is_valid());
    }
}
