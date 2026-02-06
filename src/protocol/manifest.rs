//! Protocol manifest structure and implementation
//!
//! This module contains the main ProtocolManifest structure that represents
//! a provider's protocol configuration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::config::*;
use super::error::ProtocolError;
use super::request::UnifiedRequest;

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
