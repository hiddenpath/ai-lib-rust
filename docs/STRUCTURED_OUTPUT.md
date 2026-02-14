# Structured Output in ai-lib-rust

## Overview

ai-lib-rust provides a complete structured output feature that validates AI responses against JSON schemas, ensuring type safety and adherence to expected formats. This feature is designed to achieve feature parity with ai-lib-python's structured module.

## Features

- **JSON Schema Validation**: Full support for JSON Schema specification including:
  - Type constraints (string, integer, number, boolean, array, object, null)
  - String constraints (minLength, maxLength, pattern/regex)
  - Numeric constraints (minimum, maximum)
  - Array constraints (minItems, maxItems)
  - Enum validation
  - Nested object validation
  - Required fields
  - Additional properties control

- **JSON Modes**: Three compliance levels for structured output:
  - `json_object`: Ensures valid JSON output without schema validation
  - `json_schema`: Validates against a complete JSON schema
  - `off`: No JSON enforcement

- **Schema Generation**: Automatic schema generation from Rust types using the `schemars` crate

- **OpenAI API Integration**: Direct integration with OpenAI's `response_format` parameter

- **Error Reporting**: Detailed error messages with path tracking for nested validation failures

## Module Organization

```
src/structured/
├── mod.rs          # Public API exports
├── error.rs        # Validation errors and results
├── validator.rs    # JSON schema validator implementation
├── json_mode.rs    # JSON mode configuration and result handling
└── schema.rs       # Schema generation utilities
```

## Usage Examples

### Basic JSON Object Mode

Ensures the model returns valid JSON without schema validation:

```rust
use ai_lib_rust::structured::JsonModeConfig;

// Configure JSON object mode
let config = JsonModeConfig::json_object();

// Convert to OpenAI format
let openai_format = config.to_openai_format();
// Result: {"response_format": {"type": "json_object"}}

// Use with chat request
let response = client.chat()
    .messages(vec![Message::user("Return data about a user")])
    .response_format(config)
    .execute()
    .await?;
```

### JSON Schema Validation Mode

Validates responses against a complete JSON schema:

```rust
use ai_lib_rust::structured::{JsonModeConfig, OutputValidator};
use serde_json::json;

// Define a schema
let schema = json!({
    "type": "object",
    "properties": {
        "name": {"type": "string"},
        "age": {"type": "integer", "minimum": 0},
        "email": {"type": "string", "format": "email"}
    },
    "required": ["name", "email"]
});

// Create config with schema
let config = JsonModeConfig::from_schema(schema.clone(), "User", true);

// Create validator
let validator = OutputValidator::strict(schema);

// Get and validate response
let response = client.chat()
    .messages(vec![Message::user("Create a user profile")])
    .response_format(config)
    .execute()
    .await?;

// The response is automatically validated
// If validation fails, you'll get detailed error messages
```

### Schema Generation from Rust Types

Generate JSON schemas from Rust struct definitions:

```rust
use ai_lib_rust::structured::json_schema_from_type;
use serde::Serialize;

#[derive(Serialize)]
struct UserProfile {
    name: String,
    age: u32,
    email: String,
}

// Generate schema from type
let schema = json_schema_from_type::<UserProfile>().unwrap();
```

### Manual Validation

Validate arbitrary JSON data against a schema:

```rust
use ai_lib_rust::structured::{OutputValidator, StructuredOutput};
use serde_json::json;

// Create a schema
let schema = json!({
    "type": "object",
    "properties": {
        "username": {"type": "string", "minLength": 3},
        "score": {"type": "integer", "minimum": 0, "maximum": 100}
    },
    "required": ["username", "score"]
});

// Create validator (strict or lenient mode)
let validator = OutputValidator::strict(schema);

// Validate JSON data
let result = validator.validate(json!({
    "username": "alice",
    "score": 95
}));

match result {
    Ok(validation) => println!("Valid: {:?}", validation.data),
    Err(errors) => println!("Validation errors: {:?}", errors),
}

// Or use StructuredOutput for full result handling
let output = StructuredOutput::from_response(
    r#"{"username": "alice", "score": 95}"#,
    &validator
);

if output.is_valid() {
    let parsed = output.parsed().unwrap();
    println!("Username: {}", parsed["username"]);
} else {
    for error in output.error_messages() {
        eprintln!("Error: {}", error);
    }
}
```

### Error Path Tracking

Get detailed error information with path tracking for nested objects:

```rust
let schema = json!({
    "type": "object",
    "properties": {
        "user": {
            "type": "object",
            "properties": {
                "email": {"type": "string"}
            },
            "required": ["email"]
        }
    }
});

let validator = OutputValidator::strict(schema);
let output = StructuredOutput::from_response(
    r#"{"user": {"email": 123}}"#,  // Wrong type
    &validator
);

if !output.is_valid() {
    for error in output.errors() {
        if let Some(path) = &error.path {
            eprintln!("Error at '{}': {}", path, error.message);
        } else {
            eprintln!("Error: {}", error.message);
        }
    }
}

// Output: Error at '.user.email': Expected string, but got number
```

### Integration with UnifiedRequest

The structured output is integrated into the protocol layer:

```rust
use ai_lib_rust::{AiClient, Message};
use ai_lib_rust::structured::JsonModeConfig;

let client = AiClient::new("openai/gpt-4o-mini").await?;

let config = JsonModeConfig::json_object();

// The response_format is automatically included in the unified request
let response = client.chat()
    .messages(vec![
        Message::system("You are a helpful assistant."),
        Message::user("Return a JSON object with user information")
    ])
    .response_format(config)
    .execute()
    .await?;
```

## API Reference

### JsonModeConfig

Configuration for JSON mode responses.

```rust
pub struct JsonModeConfig {
    pub mode: JsonMode,
    pub schema: Option<serde_json::Value>,
    pub name: Option<String>,
    pub strict: bool,
}
```

#### Methods

- `json_object() -> Self`: Configure JSON object mode (no schema)
- `json_schema(schema, name, strict) -> Self`: Configure with schema validation
- `off() -> Self`: Disable JSON mode
- `to_openai_format(&self) -> Value`: Convert to OpenAI API format
- `to_anthropic_format(&self) -> Value`: Convert to Anthropic format (placeholder)

### OutputValidator

Validates JSON data against a schema.

```rust
pub struct OutputValidator {
    schema: serde_json::Value,
    strict: bool,
}
```

#### Methods

- `strict(schema) -> Self`: Create strict validator (rejects additional properties)
- `lenient(schema) -> Self`: Create lenient validator (allows additional properties)
- `validate(&self, data: &Value) -> ValidationResult`: Validate JSON data

### StructuredOutput

Wraps response with validation results.

```rust
pub struct StructuredOutput {
    pub raw: String,
    pub parsed: Option<serde_json::Value>,
    pub validated: Option<serde_json::Value>,
    pub errors: Vec<ValidationError>,
}
```

#### Methods

- `from_response(raw, validator) -> Self`: Create and validate from raw response
- `from_response_unvalidated(raw) -> Self`: Create without validation
- `is_valid(&self) -> bool`: Check if validation passed
- `parsed(&self) -> Option<&Value>`: Get parsed JSON (may be `None` if parse failed)
- `validated(&self) -> Option<&Value>`: Get validated JSON (only if valid)
- `errors(&self) -> &[ValidationError]`: Get validation errors
- `error_messages(&self) -> Vec<String>`: Get error messages as strings

### ValidationError

Represents a validation error with path and context.

```rust
pub struct ValidationError {
    pub message: String,
    pub path: Option<String>,
    pub value: Option<serde_json::Value>,
    pub constraint: Option<String>,
}
```

### ValidationResult

Result type for validation operations.

```rust
pub struct ValidationResult {
    pub is_valid: bool,
    pub data: Option<serde_json::Value>,
    pub errors: Vec<ValidationError>,
}
```

## Testing

The structured output module includes comprehensive tests:

```bash
# Run all structured output tests
cargo test structured

# Run integration tests
cargo test --test structured_output_integration

# Run all tests
cargo test
```

Test coverage includes:
- 44 unit tests for internal functionality
- 7 end-to-end integration tests
- 7 doc tests for API examples

## Comparison with ai-lib-python

ai-lib-rust's structured output achieves feature parity with the Python version:

| Feature | Python | Rust |
|---------|--------|------|
| JSON Schema Validation | ✅ | ✅ |
| String Constraints | ✅ | ✅ |
| Numeric Constraints | ✅ | ✅ |
| Array Constraints | ✅ | ✅ |
| Object Validation | ✅ | ✅ |
| Nested Validation | ✅ | ✅ |
| Error Path Tracking | ✅ | ✅ |
| JSON Modes (3) | ✅ | ✅ |
| Schema from Types | ✅ | ✅ |
| OpenAI Integration | ✅ | ✅ |

## Best Practices

### 1. Use Strict Mode for Production

```rust
// Rejects unexpected fields - safer for production
let validator = OutputValidator::strict(schema);
```

### 2. Use Lenient Mode for Development

```rust
// Allows extra fields - more forgiving during development
let validator = OutputValidator::lenient(schema);
```

### 3. Handle Validation Errors Gracefully

```rust
let output = StructuredOutput::from_response(raw_response, &validator);

if !output.is_valid() {
    // Log or report errors
    for msg in output.error_messages() {
        eprintln!("Validation error: {}", msg);
    }

    // Decide whether to retry with different prompt or fail
    return Err(Error::ValidationFailed);
}

// Use validated data with confidence
let data = output.validated().unwrap();
```

### 4. Use Schema Names for Better Error Messages

```rust
let config = JsonModeConfig::from_schema(
    schema.clone(),
    "UserProfile",  // Descriptive name
    true
);
```

### 5. Combine with Type-Safe Structs

```rust
#[derive(Serialize, Deserialize)]
struct User {
    name: String,
    age: u32,
}

let schema = json_schema_from_type::<User>()?;
let config = JsonModeConfig::from_schema(schema, "User", true);

// After validation, deserialize into your struct
let user: User = serde_json::from_value(output.validated().unwrap())?;
```

## Performance Considerations

- Validation overhead is minimal for most use cases
- Schema generation from types is cached once per type
- Regex pattern matching uses compiled regexes
- Nested validation is optimized and avoids unnecessary checks

## Future Enhancements

Potential areas for future development:

- Additional JSON schema features (anyOf, allOf, oneOf, not)
- Draft 2020-12 schema support
- Custom validation rules
- Streaming response validation
- Performance benchmarks and optimizations
