//! V2 三环清单结构 — Ring1 核心骨架 / Ring2 能力映射 / Ring3 高级扩展
//!
//! V2 manifest structure implementing the concentric circle model.
//! Parses the three-ring structure from YAML/JSON and provides typed access
//! to all V2 features including MCP, Computer Use, and Extended Multimodal.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::capabilities::CapabilitiesV2;
use crate::protocol::config::{
    AccumulatorConfig, CandidateConfig, DecoderConfig, EndpointConfig, ErrorClassification,
    EventMapRule, RateLimitHeaders, RetryPolicy, ServiceConfig, TerminationConfig,
};

// ─── Ring 1: Core Skeleton (Required) ───────────────────────────────────────

/// V2 authentication configuration (Ring 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfigV2 {
    #[serde(rename = "type")]
    pub auth_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prefix: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_env: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub param_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra_headers: Option<Vec<ExtraHeader>>,
}

/// Extra header entry for authentication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraHeader {
    pub name: String,
    pub value: String,
}

/// V2 endpoint definition (Ring 1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointV2 {
    pub base_url: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub chat: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<AuthConfigV2>,
}

// ─── Ring 2: Capability Mapping (Conditional) ───────────────────────────────

/// V2 streaming configuration (Ring 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingV2 {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decoder: Option<DecoderConfig>,
    #[serde(default)]
    pub event_map: Vec<EventMapRule>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candidate: Option<CandidateConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accumulator: Option<AccumulatorConfig>,
}

/// V2 parameter definition (Ring 2).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    #[serde(rename = "type", default, skip_serializing_if = "Option::is_none")]
    pub param_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub range: Option<Vec<f64>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub alias: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

// ─── Ring 2: MCP Integration ────────────────────────────────────────────────

/// MCP integration configuration (Ring 2).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client: Option<McpClientConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub server: Option<McpServerConfig>,
}

/// MCP client configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpClientConfig {
    #[serde(default)]
    pub supported: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol_version: Option<String>,
    #[serde(default)]
    pub transports: Vec<String>,
    #[serde(default)]
    pub auth_methods: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<McpCapabilities>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_filtering: Option<McpToolFiltering>,
    #[serde(default)]
    pub approval_modes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_mapping: Option<HashMap<String, serde_json::Value>>,
}

/// MCP server capabilities that can be consumed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpCapabilities {
    #[serde(default)]
    pub tools: bool,
    #[serde(default)]
    pub resources: bool,
    #[serde(default)]
    pub prompts: bool,
    #[serde(default)]
    pub sampling: bool,
    #[serde(default)]
    pub elicitation: bool,
}

/// MCP tool filtering configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct McpToolFiltering {
    #[serde(default)]
    pub allowed_tools: bool,
    #[serde(default)]
    pub denied_tools: bool,
}

/// MCP server mode configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub transports: Vec<String>,
    #[serde(default)]
    pub exposed_capabilities: Vec<String>,
}

// ─── Ring 2: Computer Use Abstraction ───────────────────────────────────────

/// Computer Use configuration (Ring 2).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComputerUseConfig {
    #[serde(default)]
    pub supported: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub implementation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actions: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub environment: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_mapping: Option<HashMap<String, serde_json::Value>>,
}

// ─── Ring 2: Extended Multimodal ────────────────────────────────────────────

/// Extended multimodal configuration (Ring 2).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultimodalConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input: Option<MultimodalInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<MultimodalOutput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub omni_mode: Option<OmniModeConfig>,
}

/// Multimodal input modalities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultimodalInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vision: Option<VisionConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioInputConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub video: Option<VideoInputConfig>,
}

/// Vision input configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VisionConfig {
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub formats: Vec<String>,
    #[serde(default)]
    pub encoding_methods: Vec<String>,
    #[serde(default)]
    pub document_understanding: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_file_size: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_resolution: Option<String>,
}

/// Audio input configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioInputConfig {
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub formats: Vec<String>,
    #[serde(default)]
    pub real_time_streaming: bool,
    #[serde(default)]
    pub speech_recognition: bool,
}

/// Video input configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoInputConfig {
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub formats: Vec<String>,
    #[serde(default)]
    pub temporal_reasoning: bool,
    #[serde(default)]
    pub audio_track: bool,
}

/// Multimodal output modalities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MultimodalOutput {
    #[serde(default)]
    pub text: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub audio: Option<AudioOutputConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image: Option<ImageOutputConfig>,
}

/// Audio output configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioOutputConfig {
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub real_time_tts: bool,
    #[serde(default)]
    pub natural_voice: bool,
    #[serde(default)]
    pub voice_selection: bool,
}

/// Image generation output configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageOutputConfig {
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub formats: Vec<String>,
}

/// Omni-mode configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OmniModeConfig {
    #[serde(default)]
    pub supported: bool,
    #[serde(default)]
    pub real_time_voice_chat: bool,
    #[serde(default)]
    pub streaming_multimodal: bool,
}

// ─── Root V2 Manifest ───────────────────────────────────────────────────────

/// Complete V2 Provider Manifest — three-ring concentric circle structure.
///
/// Ring 1 fields are required. Ring 2 fields are conditional on capabilities.
/// Ring 3 fields are optional advanced extensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestV2 {
    // ─── Ring 1: Core Skeleton (Required) ───
    pub id: String,
    pub protocol_version: String,
    pub endpoint: EndpointV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error_classification: Option<ErrorClassification>,

    // Provider metadata
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub official_url: Option<String>,

    // ─── Ring 2: Capability Mapping (Conditional) ───
    pub capabilities: CapabilitiesV2,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parameters: Option<HashMap<String, ParameterDef>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub streaming: Option<StreamingV2>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub multimodal: Option<MultimodalConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub computer_use: Option<ComputerUseConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mcp: Option<McpConfig>,

    // ─── Ring 3: Advanced Extensions (Optional) ───
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_families: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_api_family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoints: Option<HashMap<String, EndpointConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub services: Option<HashMap<String, ServiceConfig>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limit_headers: Option<RateLimitHeaders>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry_policy: Option<RetryPolicy>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub termination: Option<TerminationConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    // Catch-all for forward compatibility
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

impl ManifestV2 {
    /// Check if the manifest declares support for a given capability.
    pub fn has_capability(&self, cap: super::capabilities::Capability) -> bool {
        self.capabilities.has_capability(cap)
    }

    /// Check if MCP client is supported.
    pub fn mcp_client_supported(&self) -> bool {
        self.mcp
            .as_ref()
            .and_then(|m| m.client.as_ref())
            .map(|c| c.supported)
            .unwrap_or(false)
    }

    /// Check if Computer Use is supported.
    pub fn computer_use_supported(&self) -> bool {
        self.computer_use
            .as_ref()
            .map(|cu| cu.supported)
            .unwrap_or(false)
    }

    /// Get the base URL for API requests.
    pub fn base_url(&self) -> &str {
        &self.endpoint.base_url
    }

    /// Get the chat endpoint path.
    pub fn chat_path(&self) -> &str {
        self.endpoint.chat.as_deref().unwrap_or("/chat/completions")
    }

    /// Detect the API style from the manifest structure.
    pub fn detect_api_style(&self) -> ApiStyle {
        // Heuristic: check streaming decoder strategy or endpoint patterns
        if let Some(streaming) = &self.streaming {
            if let Some(decoder) = &streaming.decoder {
                if let Some(strategy) = &decoder.strategy {
                    if strategy.starts_with("anthropic") {
                        return ApiStyle::AnthropicMessages;
                    }
                    if strategy.starts_with("gemini") {
                        return ApiStyle::GeminiGenerate;
                    }
                }
            }
        }
        // Check endpoint path for Gemini pattern
        if self.chat_path().contains(":generateContent") {
            return ApiStyle::GeminiGenerate;
        }
        if self.chat_path().contains("/messages") && !self.chat_path().contains("/chat/") {
            return ApiStyle::AnthropicMessages;
        }
        ApiStyle::OpenAiCompatible
    }

    /// Determine the protocol version as a semver-like tuple.
    pub fn protocol_semver(&self) -> (u32, u32) {
        let parts: Vec<&str> = self.protocol_version.split('.').collect();
        let major = parts.first().and_then(|s| s.parse().ok()).unwrap_or(1);
        let minor = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor)
    }

    /// Check if this is a V2 manifest.
    pub fn is_v2(&self) -> bool {
        self.protocol_semver().0 >= 2
    }
}

/// API style classification for ProviderDriver selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApiStyle {
    /// OpenAI chat completions format (also used by DeepSeek, Moonshot, Zhipu, etc.)
    OpenAiCompatible,
    /// Anthropic messages format
    AnthropicMessages,
    /// Google Gemini generateContent format
    GeminiGenerate,
    /// Custom format requiring a dedicated driver
    Custom,
}

impl std::fmt::Display for ApiStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpenAiCompatible => write!(f, "openai_compatible"),
            Self::AnthropicMessages => write!(f, "anthropic_messages"),
            Self::GeminiGenerate => write!(f, "gemini_generate"),
            Self::Custom => write!(f, "custom"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_v2_manifest_from_yaml() {
        let yaml = r#"
id: openai
protocol_version: "2.0"
name: OpenAI
status: stable
endpoint:
  base_url: https://api.openai.com/v1
  chat: /chat/completions
  auth:
    type: bearer
    header: Authorization
    prefix: Bearer
error_classification:
  by_http_status:
    "400": invalid_request
    "429": rate_limited
capabilities:
  required: [text, streaming, tools]
  optional: [vision, mcp_client, computer_use]
  feature_flags:
    structured_output: true
    parallel_tool_calls: true
mcp:
  client:
    supported: true
    protocol_version: "2025-11-25"
    transports: [streamable_http, sse]
computer_use:
  supported: true
  status: preview
  implementation: screen_based
streaming:
  decoder:
    format: sse
    strategy: openai_chat
"#;
        let manifest: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.id, "openai");
        assert!(manifest.is_v2());
        assert!(manifest.mcp_client_supported());
        assert!(manifest.computer_use_supported());
        assert_eq!(manifest.detect_api_style(), ApiStyle::OpenAiCompatible);
        assert!(manifest.has_capability(super::super::capabilities::Capability::McpClient));
    }

    #[test]
    fn test_detect_anthropic_style() {
        let yaml = r#"
id: anthropic
protocol_version: "2.0"
endpoint:
  base_url: https://api.anthropic.com/v1
  chat: /messages
capabilities:
  required: [text, streaming]
streaming:
  decoder:
    format: anthropic_sse
    strategy: anthropic_event_stream
"#;
        let manifest: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.detect_api_style(), ApiStyle::AnthropicMessages);
    }

    #[test]
    fn test_detect_gemini_style() {
        let yaml = r#"
id: google
protocol_version: "2.0"
endpoint:
  base_url: https://generativelanguage.googleapis.com/v1beta
  chat: "/models/{model}:generateContent"
capabilities:
  required: [text, streaming]
"#;
        let manifest: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.detect_api_style(), ApiStyle::GeminiGenerate);
    }
}
