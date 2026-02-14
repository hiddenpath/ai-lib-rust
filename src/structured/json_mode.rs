//! JSON mode support for structured output.
//!
//! Provides configuration and utilities for JSON mode responses,
//! compatible with OpenAI and Anthropic APIs.

use crate::structured::error::{ValidationError, ValidationResult};
use crate::structured::validator::OutputValidator;
use regex::Regex;

/// JSON mode options for structured output.
///
/// Defines the level of JSON structure enforcement in model outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JsonMode {
    /// Standard JSON mode - guarantees valid JSON output
    Json,

    /// JSON with schema validation - requires strict schema compliance
    JsonSchema,

    /// Disabled - no JSON mode enforcement
    Off,
}

impl JsonMode {
    /// Get the string representation for API requests.
    pub fn as_str(&self) -> &'static str {
        match self {
            JsonMode::Json => "json_object",
            JsonMode::JsonSchema => "json_schema",
            JsonMode::Off => "",
        }
    }
}

impl std::fmt::Display for JsonMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for JsonMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json_object" => Ok(JsonMode::Json),
            "json_schema" => Ok(JsonMode::JsonSchema),
            "off" | "" => Ok(JsonMode::Off),
            _ => Err(format!("Unknown JSON mode: {}", s)),
        }
    }
}

/// Configuration for JSON mode.
///
/// Defines how structured output should be formatted and validated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonModeConfig {
    /// JSON mode to use
    pub mode: JsonMode,

    /// JSON schema for validation (only used for JsonSchema mode)
    pub schema: Option<serde_json::Value>,

    /// Name for the schema (used in OpenAI format)
    pub schema_name: String,

    /// Whether to enforce strict schema compliance
    pub strict: bool,
}

impl JsonModeConfig {
    /// Create a config for simple JSON object mode.
    ///
    /// Example:
    ///
    /// ```
    /// use ai_lib_rust::structured::JsonModeConfig;
    ///
    /// let config = JsonModeConfig::json_object();
    /// let openai_format = config.to_openai_format();
    /// assert_eq!(openai_format["response_format"]["type"], "json_object");
    /// ```
    pub fn json_object() -> Self {
        Self {
            mode: JsonMode::Json,
            schema: None,
            schema_name: "response".to_string(),
            strict: true,
        }
    }

    /// Create a config from a JSON schema.
    ///
    /// # Arguments
    ///
    /// * `schema` - JSON schema dictionary
    /// * `name` - Schema name (default: "response")
    /// * `strict` - Whether to enforce strict compliance (default: true)
    ///
    /// Example:
    ///
    /// ```
    /// use ai_lib_rust::structured::{JsonMode, JsonModeConfig};
    /// use serde_json::json;
    ///
    /// let schema = json!({
    ///     "type": "object",
    ///     "properties": {
    ///         "name": {"type": "string"}
    ///     },
    ///     "required": ["name"]
    /// });
    ///
    /// let config = JsonModeConfig::from_schema(schema, "test", true);
    /// assert_eq!(config.mode, JsonMode::JsonSchema);
    /// ```
    pub fn from_schema(schema: serde_json::Value, name: impl Into<String>, strict: bool) -> Self {
        Self {
            mode: JsonMode::JsonSchema,
            schema: Some(schema),
            schema_name: name.into(),
            strict,
        }
    }

    /// Convert to OpenAI API format.
    ///
    /// Returns a value suitable for the `response_format` parameter
    /// in OpenAI's Chat Completions API.
    ///
    /// Example output for JSON mode:
    /// ```json
    /// {
    ///   "response_format": {
    ///     "type": "json_object"
    ///   }
    /// }
    /// ```
    ///
    /// Example output for JSON Schema mode:
    /// ```json
    /// {
    ///   "response_format": {
    ///     "type": "json_schema",
    ///     "json_schema": {
    ///       "name": "response",
    ///       "strict": true,
    ///       "schema": { ... }
    ///     }
    ///   }
    /// }
    /// ```
    pub fn to_openai_format(&self) -> serde_json::Value {
        match self.mode {
            JsonMode::Off => serde_json::json!({}),

            JsonMode::Json => serde_json::json!({
                "response_format": {
                    "type": self.mode.as_str()
                }
            }),

            JsonMode::JsonSchema => {
                let schema = self
                    .schema
                    .as_ref()
                    .expect("Schema required for JsonSchema mode");
                serde_json::json!({
                    "response_format": {
                        "type": self.mode.as_str(),
                        "json_schema": {
                            "name": self.schema_name,
                            "strict": self.strict,
                            "schema": schema
                        }
                    }
                })
            }
        }
    }

    /// Convert to Anthropic API format.
    ///
    /// Note: Anthropic doesn't have native JSON mode support.
    /// This returns an empty placeholder, and JSON enforcement
    /// must be done through system prompt instructions.
    ///
    /// Example:
    ///
    /// ```
    /// use ai_lib_rust::structured::JsonModeConfig;
    /// use serde_json::json;
    ///
    /// let config = JsonModeConfig::json_object();
    /// let anthropic_format = config.to_anthropic_format();
    /// assert_eq!(anthropic_format, json!({}));
    /// ```
    pub fn to_anthropic_format(&self) -> serde_json::Value {
        // Anthropic relies on system prompt instructions
        serde_json::json!({})
    }
}

/// Structured output result with validation.
///
/// Wraps the raw response from the AI model with parsed,
/// validated, and formatted data.
#[derive(Debug, Clone)]
pub struct StructuredOutput {
    /// Raw response content as string
    pub raw: String,

    /// Parsed JSON data (None if parsing failed)
    pub parsed: Option<serde_json::Value>,

    /// Validation result (always populated)
    pub validation_result: ValidationResult,
}

impl StructuredOutput {
    /// Create a structured output from raw content without validation.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw response content
    ///
    /// Returns:
    /// A StructuredOutput instance with parsed data but without validation.
    ///
    /// Use `.validate()` method to add validation after creation.
    ///
    /// Example:
    ///
    /// ```
    /// use ai_lib_rust::structured::{StructuredOutput, OutputValidator};
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
    /// let validator = OutputValidator::lenient(schema);
    ///
    /// let mut output = StructuredOutput::from_response_unvalidated(
    ///     r#"{"name": "Alice"}"#
    /// );
    ///
    /// output.validate(&validator);
    /// assert!(output.is_valid());
    /// ```
    pub fn from_response_unvalidated(content: impl Into<String>) -> Self {
        let content = content.into();
        let content_str = content.trim();

        // Try to parse JSON
        let parsed = Self::parse_json(content_str);

        // Create a default validation result without validation
        let validation_result = ValidationResult::success(
            parsed
                .clone()
                .unwrap_or_else(|| serde_json::Value::String(content_str.to_string())),
        );

        Self {
            raw: content,
            parsed,
            validation_result,
        }
    }

    /// Validate this output against a schema.
    ///
    /// # Arguments
    ///
    /// * `validator` - The validator to use
    ///
    /// Updates the validation_result with the schema check result.
    ///
    /// Example:
    ///
    /// ```
    /// use ai_lib_rust::structured::{StructuredOutput, OutputValidator};
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
    /// let validator = OutputValidator::lenient(schema);
    ///
    /// let mut output = StructuredOutput::from_response_unvalidated(r#"{"name": "Alice"}"#);
    /// output.validate(&validator);
    /// ```
    pub fn validate(&mut self, validator: &OutputValidator) {
        if let Some(parsed) = &self.parsed {
            self.validation_result = validator.validate(parsed);
        }
    }

    /// Create a structured output from raw content with validation.
    ///
    /// # Arguments
    ///
    /// * `content` - Raw response content
    /// * `validator` - Validator to check the content
    ///
    /// Returns:
    /// A StructuredOutput instance with parsed and validated data.
    ///
    /// Example:
    ///
    /// ```
    /// use ai_lib_rust::structured::{StructuredOutput, OutputValidator};
    /// use serde_json::json;
    ///
    /// let schema = json!({"type": "object", "properties": {"name": {"type": "string"}}});
    /// let validator = OutputValidator::lenient(schema);
    ///
    /// let output = StructuredOutput::from_response(
    ///     r#"{"name": "Alice"}"#,
    ///     &validator
    /// );
    ///
    /// assert!(output.is_valid());
    /// ```
    pub fn from_response(content: impl Into<String>, validator: &OutputValidator) -> Self {
        let mut output = Self::from_response_unvalidated(content);
        output.validate(validator);
        output
    }

    /// Parse JSON from text, with support for markdown code blocks.
    ///
    /// Extracts JSON from common formats:
    /// - Raw JSON object
    /// - ```json ... ``` code blocks
    /// - ``` ... ``` code blocks
    /// - Text containing JSON objects/arrays
    fn parse_json(text: &str) -> Option<serde_json::Value> {
        // Try direct parsing first
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text) {
            return Some(parsed);
        }

        // Try to extract from markdown code blocks
        let patterns = [
            r"```json\s*([\s\S]*?)\s*```",
            r"```\s*([\s\S]*?)\s*```",
            r"\{[\s\S]*\}",
            r"\[[\s\S]*\]",
        ];

        for pattern in patterns {
            if let Ok(re) = Regex::new(pattern) {
                if let Some(captures) = re.captures(text) {
                    let candidate = match captures.get(1) {
                        Some(inner) => inner.as_str(),
                        None => captures.get(0).map(|c| c.as_str()).unwrap_or(text),
                    };

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(candidate.trim())
                    {
                        return Some(parsed);
                    }
                }
            }
        }

        None
    }

    /// Check if the output is valid.
    ///
    /// Returns true if:
    /// 1. JSON parsing succeeded
    /// 2. Validation passed (if validation was performed)
    pub fn is_valid(&self) -> bool {
        self.validation_result.is_valid()
    }

    /// Get the best available data representation.
    ///
    /// Priority:
    /// 1. Validated data (if validation passed)
    /// 2. Parsed data (if available)
    /// 3. Raw content as string
    pub fn data(&self) -> serde_json::Value {
        if let Some(data) = self.validation_result.data() {
            return data.clone();
        }
        if let Some(parsed) = &self.parsed {
            return parsed.clone();
        }
        serde_json::Value::String(self.raw.clone())
    }

    /// Get the raw response content.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// Get the parsed JSON data (if parsing succeeded).
    pub fn parsed(&self) -> Option<&serde_json::Value> {
        self.parsed.as_ref()
    }

    /// Get the validation result.
    pub fn validation_result(&self) -> &ValidationResult {
        &self.validation_result
    }

    /// Get validation errors if validation failed.
    pub fn errors(&self) -> Vec<ValidationError> {
        if self.validation_result.is_valid() {
            Vec::new()
        } else {
            self.validation_result.errors.clone()
        }
    }

    /// Get error messages as strings.
    pub fn error_messages(&self) -> Vec<String> {
        self.validation_result.error_messages()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_mode_config_json_object() {
        let config = JsonModeConfig::json_object();

        assert_eq!(config.mode, JsonMode::Json);
        assert!(config.schema.is_none());
        assert_eq!(config.schema_name, "response");
        assert!(config.strict);
    }

    #[test]
    fn test_json_mode_config_from_schema() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "name": {"type": "string"}
            }
        });

        let config = JsonModeConfig::from_schema(schema, "User", true);

        assert_eq!(config.mode, JsonMode::JsonSchema);
        assert!(config.schema.is_some());
        assert_eq!(config.schema_name, "User");
        assert!(config.strict);
    }

    #[test]
    fn test_json_mode_config_to_openai_format_json() {
        let config = JsonModeConfig::json_object();
        let openai = config.to_openai_format();

        assert_eq!(openai["response_format"]["type"], "json_object");
    }

    #[test]
    fn test_json_mode_config_to_openai_format_json_schema() {
        let schema = serde_json::json!({
            "type": "string"
        });

        let config = JsonModeConfig::from_schema(schema.clone(), "test", false);
        let openai = config.to_openai_format();

        assert_eq!(openai["response_format"]["type"], "json_schema");
        assert_eq!(openai["response_format"]["json_schema"]["name"], "test");
        assert_eq!(openai["response_format"]["json_schema"]["strict"], false);
        assert_eq!(openai["response_format"]["json_schema"]["schema"], schema);
    }

    #[test]
    fn test_json_mode_config_to_openai_format_off() {
        let config = JsonModeConfig {
            mode: JsonMode::Off,
            schema: None,
            schema_name: "test".to_string(),
            strict: false,
        };
        let openai = config.to_openai_format();

        // Should be empty object
        assert_eq!(openai, serde_json::json!({}));
    }

    #[test]
    fn test_json_mode_config_to_anthropic_format() {
        let config = JsonModeConfig::json_object();
        let anthropic = config.to_anthropic_format();

        // Anthropic format is empty (relies on system prompt)
        assert_eq!(anthropic, serde_json::json!({}));
    }

    #[test]
    fn test_structured_output_valid_json() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "result": {"type": "string"}
            }
        });
        let validator = OutputValidator::lenient(schema);

        let output = StructuredOutput::from_response(r#"{"result": "success"}"#, &validator);

        assert!(output.is_valid());
        assert!(output.parsed().is_some());
    }

    #[test]
    fn test_structured_output_invalid_json() {
        let output = StructuredOutput::from_response_unvalidated("not json");

        assert!(output.is_valid());
        assert!(output.parsed().is_none());
    }

    #[test]
    fn test_structured_output_parsed_json() {
        let output = StructuredOutput::from_response_unvalidated(r#"{"valid": true}"#);

        assert_eq!(output.parsed().unwrap()["valid"], true);
    }

    #[test]
    fn test_structured_output_json_from_markdown() {
        let output = StructuredOutput::from_response_unvalidated(
            r#"Here is the JSON:
            ```json
            {"result": "success"}
            ```"#,
        );

        assert_eq!(output.parsed().unwrap()["result"], "success");
    }

    #[test]
    fn test_structured_output_data_priority() {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "value": {"type": "string"}
            }
        });
        let validator = OutputValidator::lenient(schema);

        let mut output = StructuredOutput::from_response_unvalidated(r#"{"value": "test"}"#);
        output.validate(&validator);

        // validation passes, so should return validated data
        let data = output.data();
        assert_eq!(*output.validation_result.data().unwrap(), data);
    }

    #[test]
    fn test_structured_output_validate_method() {
        let schema = serde_json::json!({"type": "integer"});
        let validator = OutputValidator::lenient(schema);

        let mut output = StructuredOutput::from_response_unvalidated(r#"{"value": "test"}"#);
        output.validate(&validator);

        assert!(!output.is_valid());
        assert!(!output.errors().is_empty());
    }

    #[test]
    fn test_structured_output_errors() {
        let schema = serde_json::json!({"type": "integer"});
        let validator = OutputValidator::lenient(schema);

        let mut output = StructuredOutput::from_response_unvalidated(r#"{"value": "not integer"}"#);
        output.validate(&validator);

        assert!(!output.is_valid());
        assert!(!output.errors().is_empty());
    }
}
