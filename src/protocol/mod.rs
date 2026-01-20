//! Protocol specification layer
//!
//! This module handles loading, validating, and managing AI-Protocol specifications.
//! It provides the foundation for the protocol-driven architecture.

pub mod loader;
pub mod schema;
pub mod validator;

pub use loader::ProtocolLoader;
pub use schema::ProtocolSchema;
pub use validator::ProtocolValidator;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unified request format (for protocol compilation)
#[derive(Debug, Clone, Default)]
pub struct UnifiedRequest {
    /// Operation intent used for endpoint routing (e.g. "chat", "completions", "embeddings")
    pub operation: String,
    /// Provider model id (e.g. "deepseek-chat", "gpt-4o-mini")
    pub model: String,
    pub messages: Vec<crate::types::message::Message>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub tools: Option<Vec<crate::types::tool::ToolDefinition>>,
    /// OpenAI-style tool choice. Examples:
    /// - "auto"
    /// - "none"
    /// - {"type":"function","function":{"name":"web_search"}}
    pub tool_choice: Option<serde_json::Value>,
}

/// Protocol error types
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Failed to load protocol from {path}: {reason}{}", .hint.as_ref().map(|h| format!("\n💡 Hint: {}", h)).unwrap_or_default())]
    LoadError {
        path: String,
        reason: String,
        hint: Option<String>,
    },

    #[error("Protocol validation failed: {0}")]
    ValidationError(String),

    #[error("Schema mismatch: expected {expected}, found {actual} at {path}{}", .hint.as_ref().map(|h| format!("\n💡 Hint: {}", h)).unwrap_or_default())]
    SchemaMismatch {
        path: String,
        expected: String,
        actual: String,
        hint: Option<String>,
    },

    #[error("Protocol not found: {id}{}", .hint.as_ref().map(|h| format!("\n💡 Hint: {}", h)).unwrap_or_default())]
    NotFound { id: String, hint: Option<String> },

    #[error("Unsupported protocol version '{version}' (max supported: {max_supported}){}", .hint.as_ref().map(|h| format!("\n💡 Hint: {}", h)).unwrap_or_default())]
    InvalidVersion {
        version: String,
        max_supported: String,
        hint: Option<String>,
    },

    #[error("Configuration manifest error: {0}")]
    ManifestError(String),

    #[error("Internal protocol error: {0}")]
    Internal(String),

    #[error("YAML syntax error: {0}")]
    YamlError(String),
}

impl ProtocolError {
    /// Attach an actionable hint to the error
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        let hint_val = Some(hint.into());
        match self {
            ProtocolError::LoadError { ref mut hint, .. } => *hint = hint_val,
            ProtocolError::SchemaMismatch { ref mut hint, .. } => *hint = hint_val,
            ProtocolError::NotFound { ref mut hint, .. } => *hint = hint_val,
            ProtocolError::InvalidVersion { ref mut hint, .. } => *hint = hint_val,
            _ => (),
        }
        self
    }
}

/// Protocol manifest structure (parsed from YAML)
///
/// Required fields per schema: id, protocol_version, endpoint, availability, capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolManifest {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    // Required fields
    pub id: String,
    pub protocol_version: String,
    pub endpoint: EndpointDefinition,
    pub availability: AvailabilityConfig,
    pub capabilities: Capabilities,

    // Provider metadata (required in manifests)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    pub status: String,   // stable/beta/deprecated
    pub category: String, // ai_provider / model_provider / third_party_aggregator
    pub official_url: String,
    pub support_contact: String,

    // Auth and configuration
    pub auth: AuthConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub payload_format: Option<String>,
    pub parameter_mappings: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_paths: Option<HashMap<String, String>>,

    // Streaming and features
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<StreamingConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub features: Option<FeaturesConfig>,

    // Endpoints and services
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<HashMap<String, EndpointConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub services: Option<HashMap<String, ServiceConfig>>,

    // API families
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_families: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_api_family: Option<String>,

    // Tooling and termination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub termination: Option<TerminationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tooling: Option<ToolingConfig>,

    // Error handling and resilience
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<RetryPolicy>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_classification: Option<ErrorClassification>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limit_headers: Option<RateLimitHeaders>,

    // Experimental features
    #[serde(skip_serializing_if = "Option::is_none")]
    pub experimental_features: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EndpointConfig {
    pub path: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
}

impl<'de> Deserialize<'de> for EndpointConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Input {
            // Shorthand: endpoint: "/v1/chat/completions"
            Path(String),
            // Full form
            Obj {
                path: String,
                #[serde(default = "default_method")]
                method: String,
                #[serde(default)]
                adapter: Option<String>,
            },
        }

        match Input::deserialize(deserializer)? {
            Input::Path(path) => Ok(EndpointConfig {
                path,
                method: default_method(),
                adapter: None,
            }),
            Input::Obj {
                path,
                method,
                adapter,
            } => Ok(EndpointConfig {
                path,
                method,
                adapter,
            }),
        }
    }
}

fn default_method() -> String {
    "POST".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub path: String,
    #[serde(default = "default_method_get")]
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query_params: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_binding: Option<String>,
}

fn default_method_get() -> String {
    "GET".to_string()
}

/// Structured endpoint definition (v1.1+ extension)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointDefinition {
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>, // https, http, ws, wss
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
}

/// Capabilities object format (v1.1+)
/// Required fields: streaming, tools, vision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    pub streaming: bool,
    pub tools: bool,
    pub vision: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub agentic: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub parallel_tools: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub reasoning: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub multimodal: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub audio: bool,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl ProtocolManifest {
    /// Check if protocol supports a specific capability
    pub fn supports_capability(&self, capability: &str) -> bool {
        match capability {
            "streaming" => self.capabilities.streaming,
            "tools" => self.capabilities.tools,
            "vision" => self.capabilities.vision,
            "agentic" => self.capabilities.agentic,
            "parallel_tools" => self.capabilities.parallel_tools,
            "reasoning" => self.capabilities.reasoning,
            "multimodal" => {
                self.capabilities.multimodal || self.capabilities.vision || self.capabilities.audio
            }
            "audio" => self.capabilities.audio,
            _ => false,
        }
    }

    /// Get base URL from endpoint definition
    pub fn get_base_url(&self) -> &str {
        &self.endpoint.base_url
    }

    /// Compile unified request to provider-specific format
    pub fn compile_request(
        &self,
        request: &UnifiedRequest,
    ) -> Result<serde_json::Value, ProtocolError> {
        use crate::utils::PathMapper;

        let mut provider_request = serde_json::json!({});

        // Model is required for most OpenAI-compatible APIs
        let model_path = self
            .parameter_mappings
            .get("model")
            .map(|s| s.as_str())
            .unwrap_or("model");
        PathMapper::set_path(
            &mut provider_request,
            model_path,
            serde_json::Value::String(request.model.clone()),
        )
        .map_err(|e| ProtocolError::ValidationError(format!("Failed to set model: {}", e)))?;

        // Map standard parameters to provider-specific names using PathMapper
        if let Some(temp) = request.temperature {
            if let Some(mapped) = self.parameter_mappings.get("temperature") {
                PathMapper::set_path(
                    &mut provider_request,
                    mapped,
                    serde_json::Value::Number(serde_json::Number::from_f64(temp).ok_or_else(
                        || ProtocolError::ValidationError("Invalid temperature".to_string()),
                    )?),
                )
                .map_err(|e| {
                    ProtocolError::ValidationError(format!("Failed to set temperature: {}", e))
                })?;
            }
        }

        if let Some(max) = request.max_tokens {
            if let Some(mapped) = self.parameter_mappings.get("max_tokens") {
                PathMapper::set_path(
                    &mut provider_request,
                    mapped,
                    serde_json::Value::Number(max.into()),
                )
                .map_err(|e| {
                    ProtocolError::ValidationError(format!("Failed to set max_tokens: {}", e))
                })?;
            }
        }

        if let Some(mapped) = self.parameter_mappings.get("stream") {
            PathMapper::set_path(
                &mut provider_request,
                mapped,
                serde_json::Value::Bool(request.stream),
            )
            .map_err(|e| ProtocolError::ValidationError(format!("Failed to set stream: {}", e)))?;
        }

        // Map messages (format depends on payload_format)
        let messages_path = self
            .parameter_mappings
            .get("messages")
            .map(|s| s.as_str())
            .unwrap_or("messages");
        let messages: Vec<serde_json::Value> = request
            .messages
            .iter()
            .map(|m| serde_json::to_value(m).unwrap())
            .collect();
        PathMapper::set_path(
            &mut provider_request,
            messages_path,
            serde_json::Value::Array(messages),
        )
        .map_err(|e| ProtocolError::ValidationError(format!("Failed to set messages: {}", e)))?;

        // Map tools if present
        if let Some(tools) = &request.tools {
            if let Some(mapped) = self.parameter_mappings.get("tools") {
                let tools_value: Vec<serde_json::Value> = tools
                    .iter()
                    .map(|t| serde_json::to_value(t).unwrap())
                    .collect();
                PathMapper::set_path(
                    &mut provider_request,
                    mapped,
                    serde_json::Value::Array(tools_value),
                )
                .map_err(|e| {
                    ProtocolError::ValidationError(format!("Failed to set tools: {}", e))
                })?;
            }
        }

        // Map tool_choice if present
        if let Some(tool_choice) = &request.tool_choice {
            if let Some(mapped) = self.parameter_mappings.get("tool_choice") {
                PathMapper::set_path(&mut provider_request, mapped, tool_choice.clone()).map_err(
                    |e| ProtocolError::ValidationError(format!("Failed to set tool_choice: {}", e)),
                )?;
            }
        }

        Ok(provider_request)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(rename = "type")]
    pub auth_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key_env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_headers: Option<Vec<HeaderConfig>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub decoder: Option<DecoderConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_selector: Option<String>,
    /// Common path for content delta in streaming frames (provider-specific)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_path: Option<String>,
    /// Common path for tool call delta in streaming frames (provider-specific)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_path: Option<String>,
    /// Common path for usage metadata in streaming frames (provider-specific)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate: Option<CandidateConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accumulator: Option<AccumulatorConfig>,
    #[serde(default)]
    pub event_map: Vec<EventMapRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_condition: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecoderConfig {
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strategy: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_signal: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_id_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fan_out: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulatorConfig {
    #[serde(default)]
    pub stateful_tool_parsing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flush_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMapRule {
    #[serde(rename = "match")]
    pub match_expr: String,
    pub emit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi_candidate: Option<MultiCandidateConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_mapping: Option<ResponseMappingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCandidateConfig {
    pub support_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMappingConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<ToolCallsMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallsMapping {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    pub fields: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub array_fan_out: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMapping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationConfig {
    pub source_field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolingConfig {
    pub source_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_use: Option<ToolUseMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResultMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolUseMapping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_format: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultMapping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    pub strategy: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_retries: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min_delay_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_delay_ms: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub jitter: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_on_http_status: Option<Vec<u16>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_on_error_status: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorClassification {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by_http_status: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub by_error_status: Option<HashMap<String, String>>,
}

/// Availability and health checking configuration (v1.1+ extension)
/// Required fields: required, regions, check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AvailabilityConfig {
    pub required: bool,
    pub regions: Vec<String>, // cn, global, us, eu
    pub check: HealthCheckConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<Vec<String>>,
}

/// Health check endpoint configuration
/// Required fields: method, path, expected_status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    pub method: String, // HEAD, GET
    pub path: String,
    pub expected_status: Vec<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitHeaders {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests_limit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests_remaining: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requests_reset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_limit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_remaining: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_reset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_after: Option<String>,
}
