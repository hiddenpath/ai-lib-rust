//! Protocol configuration structures
//!
//! This module contains all the configuration-related structures used in protocol manifests.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Structured endpoint definition (v1.1+ extension)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointDefinition {
    pub base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>, // https, http, ws, wss
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u32>,
}

/// Endpoint configuration for specific operations
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

/// Service configuration for auxiliary endpoints
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

/// Capabilities object format (v1.1+ legacy + v2 required/optional shape)
#[derive(Debug, Clone, Serialize)]
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
    /// Structured output / JSON mode (capability or `feature_flags.structured_output`).
    #[serde(default, skip_serializing_if = "is_false")]
    pub structured_output: bool,
    /// MCP client tool-bridge (`mcp_client` in V2 optional/required lists).
    #[serde(default, skip_serializing_if = "is_false")]
    pub mcp_client: bool,
}

impl<'de> Deserialize<'de> for Capabilities {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct LegacyCaps {
            streaming: bool,
            tools: bool,
            vision: bool,
            #[serde(default)]
            agentic: bool,
            #[serde(default)]
            parallel_tools: bool,
            #[serde(default)]
            reasoning: bool,
            #[serde(default)]
            multimodal: bool,
            #[serde(default)]
            audio: bool,
        }

        #[derive(Deserialize, Default)]
        struct FeatureFlags {
            #[serde(default)]
            parallel_tool_calls: bool,
            #[serde(default)]
            extended_thinking: bool,
            #[serde(default)]
            structured_output: bool,
        }

        #[derive(Deserialize)]
        struct V2Caps {
            required: Vec<String>,
            #[serde(default)]
            optional: Vec<String>,
            #[serde(default)]
            feature_flags: Option<FeatureFlags>,
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Input {
            /// Shorthand list e.g. `[chat, streaming, tools]` in compliance fixtures.
            TagList(Vec<String>),
            Legacy(LegacyCaps),
            V2(V2Caps),
        }

        fn from_capability_tags(tags: &[String]) -> Capabilities {
            let mut c = Capabilities {
                streaming: false,
                tools: false,
                vision: false,
                agentic: false,
                parallel_tools: false,
                reasoning: false,
                multimodal: false,
                audio: false,
                structured_output: false,
                mcp_client: false,
            };
            for t in tags {
                match t.as_str() {
                    "chat" | "text" => {}
                    "streaming" => c.streaming = true,
                    "tools" => c.tools = true,
                    "vision" => {
                        c.vision = true;
                        c.multimodal = true;
                    }
                    "audio" => {
                        c.audio = true;
                        c.multimodal = true;
                    }
                    "video" => c.multimodal = true,
                    "agentic" => c.agentic = true,
                    "parallel_tools" => c.parallel_tools = true,
                    "reasoning" => c.reasoning = true,
                    "structured_output" => c.structured_output = true,
                    "mcp_client" => c.mcp_client = true,
                    _ => {}
                }
            }
            c
        }

        match Input::deserialize(deserializer)? {
            Input::TagList(tags) => Ok(from_capability_tags(&tags)),
            Input::Legacy(v) => Ok(Capabilities {
                streaming: v.streaming,
                tools: v.tools,
                vision: v.vision,
                agentic: v.agentic,
                parallel_tools: v.parallel_tools,
                reasoning: v.reasoning,
                multimodal: v.multimodal,
                audio: v.audio,
                structured_output: false,
                mcp_client: false,
            }),
            Input::V2(v) => {
                let has = |name: &str| {
                    v.required.iter().any(|c| c == name) || v.optional.iter().any(|c| c == name)
                };
                let flags = v.feature_flags.unwrap_or_default();
                Ok(Capabilities {
                    streaming: has("streaming"),
                    tools: has("tools"),
                    vision: has("vision"),
                    agentic: has("agentic"),
                    parallel_tools: has("parallel_tools") || flags.parallel_tool_calls,
                    reasoning: has("reasoning") || flags.extended_thinking,
                    multimodal: has("vision") || has("audio") || has("video"),
                    audio: has("audio"),
                    structured_output: has("structured_output") || flags.structured_output,
                    mcp_client: has("mcp_client"),
                })
            }
        }
    }
}

fn is_false(b: &bool) -> bool {
    !*b
}

/// Authentication configuration
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

/// Header configuration for extra headers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderConfig {
    pub name: String,
    pub value: String,
}

/// Streaming configuration
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

/// Decoder configuration for streaming
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

/// Candidate configuration for multi-candidate responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub candidate_id_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fan_out: Option<bool>,
}

/// Accumulator configuration for stateful parsing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccumulatorConfig {
    #[serde(default)]
    pub stateful_tool_parsing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flush_on: Option<String>,
}

/// Event mapping rule for streaming events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMapRule {
    #[serde(rename = "match")]
    pub match_expr: String,
    pub emit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fields: Option<HashMap<String, String>>,
}

/// Features configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multi_candidate: Option<MultiCandidateConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_mapping: Option<ResponseMappingConfig>,
}

/// Multi-candidate configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCandidateConfig {
    pub support_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_concurrent: Option<u32>,
}

/// Response mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMappingConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<ToolCallsMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorMapping>,
}

/// Tool calls mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallsMapping {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<String>,
    pub fields: HashMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub array_fan_out: Option<bool>,
}

/// Error mapping configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorMapping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub type_path: Option<String>,
}

/// Termination configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TerminationConfig {
    pub source_field: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapping: Option<HashMap<String, String>>,
}

/// Tooling configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolingConfig {
    pub source_model: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_use: Option<ToolUseMapping>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<ToolResultMapping>,
}

/// Tool use mapping
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

/// Tool result mapping
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResultMapping {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub response_path: Option<String>,
}

/// Retry policy configuration
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

/// Error classification configuration
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

/// Rate limit headers configuration
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
