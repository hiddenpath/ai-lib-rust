//! Integration tests for structured output functionality

use ai_lib_rust::structured::{
    JsonModeConfig, OutputValidator, SchemaGenerator, StructuredOutput, ValidationError,
    ValidationResult,
};
use serde_json::json;

#[test]
fn test_end_to_end_json_mode_schema_validation() {
    // Create a schema for a user object
    let schema = SchemaGenerator::new()
        .title("User")
        .description("A user object")
        .add_property("name", json!({"type": "string"}))
        .add_property("age", json!({"type": "integer"}))
        .set_required(&vec!["name".to_string(), "age".to_string()]);

    let json_schema = schema.build();

    // Create JSON mode config with schema
    let config = JsonModeConfig::from_schema(json_schema.clone(), "User", true);

    // Convert to OpenAI format
    let openai_format = config.to_openai_format();
    assert_eq!(openai_format["response_format"]["type"], "json_schema");
    assert_eq!(
        openai_format["response_format"]["json_schema"]["name"],
        "User"
    );
    assert_eq!(
        openai_format["response_format"]["json_schema"]["strict"],
        true
    );

    // Test validation with data
    let validator = OutputValidator::strict(json_schema);
    let output = StructuredOutput::from_response(r#"{"name": "Alice", "age": 30}"#, &validator);

    assert!(output.is_valid());
    assert_eq!(output.parsed().unwrap()["name"], "Alice");
}

#[test]
fn test_end_to_end_json_object_mode() {
    let config = JsonModeConfig::json_object();
    let openai_format = config.to_openai_format();

    assert_eq!(openai_format["response_format"]["type"], "json_object");

    // Validate JSON without schema (should accept any valid JSON)
    let output =
        StructuredOutput::from_response_unvalidated(r#"{"status": "success", "count": 42}"#);

    assert!(output.parsed().is_some());
    assert_eq!(output.parsed().unwrap()["count"], 42);
}

#[test]
fn test_invalid_data_validation() {
    let schema = json!({
        "type": "object",
        "properties": {
            "value": {"type": "integer"}
        },
        "required": ["value"]
    });

    let validator = OutputValidator::strict(schema);

    // Test with wrong type
    let output = StructuredOutput::from_response(r#"{"value": "not_an_integer"}"#, &validator);

    assert!(!output.is_valid());
    assert!(!output.errors().is_empty());
    assert!(output.error_messages()[0].contains("Expected"));
}

#[test]
fn test_nested_schema_validation() {
    let schema = json!({
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

    // Valid nested data
    let output = StructuredOutput::from_response(r#"{"user": {"name": "Bob"}}"#, &validator);

    assert!(output.is_valid());

    // Missing required nested field
    let output = StructuredOutput::from_response(r#"{"user": {}}"#, &validator);

    assert!(!output.is_valid());
    assert!(output.error_messages()[0].contains("Missing required"));
}

#[test]
fn test_array_validation() {
    let schema = json!({
        "type": "array",
        "items": {
            "type": "string",
            "minLength": 2
        },
        "minItems": 1,
        "maxItems": 3
    });

    let validator = OutputValidator::lenient(schema);

    // Valid array
    let output = StructuredOutput::from_response(r#"["ab", "cd", "ef"]"#, &validator);

    assert!(output.is_valid());

    // Array too short
    let output = StructuredOutput::from_response(r#"["a"]"#, &validator);
    assert!(!output.is_valid());

    // Too many items
    let output = StructuredOutput::from_response(r#"["a", "b", "c", "d"]"#, &validator);
    assert!(!output.is_valid());
}

#[test]
fn test_error_paths_in_nested_objects() {
    let schema = json!({
        "type": "object",
        "properties": {
            "user": {
                "type": "object",
                "properties": {
                    "email": {"type": "string"}
                }
            }
        }
    });

    let validator = OutputValidator::strict(schema);

    // Test with path to error
    let output = StructuredOutput::from_response(r#"{"user": {"email": 123}}"#, &validator);

    assert!(!output.is_valid());
    let errors = output.errors();
    assert!(!errors.is_empty());
    assert!(errors[0].path.as_ref().unwrap().contains("user"));
}

#[test]
fn test_validation_result_merge() {
    // Test error merging
    let result1 = ValidationResult::from_error(ValidationError::without_path("Error 1"));
    let result2 = ValidationResult::from_error(ValidationError::without_path("Error 2"));

    let merged = ValidationResult::merge(vec![result1, result2]);
    assert!(!merged.is_valid());
    assert_eq!(merged.errors.len(), 2);

    // Test success merge
    let result3 = ValidationResult::success(json!({
        "test": "data"
    }));
    let result4 = ValidationResult::success(json!({
        "test2": "data2"
    }));

    let merged = ValidationResult::merge(vec![result3, result4]);
    assert!(merged.is_valid());
}
