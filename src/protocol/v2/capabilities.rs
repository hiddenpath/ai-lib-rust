//! V2 能力声明系统 — 支持 required/optional 分离和 feature_flags 精细控制
//!
//! V2 capability declaration system with structured required/optional separation,
//! feature flags, and capability-to-module mapping for runtime loading.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Standard capability identifiers aligned with `schemas/v2/capabilities.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Text,
    Streaming,
    Vision,
    Audio,
    Video,
    Tools,
    ParallelTools,
    Agentic,
    Reasoning,
    Embeddings,
    StructuredOutput,
    Batch,
    ImageGeneration,
    ComputerUse,
    McpClient,
    McpServer,
}

impl Capability {
    /// Map capability to the corresponding Cargo feature flag name.
    pub fn feature_flag(&self) -> Option<&'static str> {
        match self {
            Self::Text | Self::Streaming | Self::Tools | Self::ParallelTools => None, // always loaded
            Self::Vision => Some("vision"),
            Self::Audio | Self::Video => Some("multimodal"),
            Self::Agentic => Some("agentic"),
            Self::Reasoning => Some("reasoning"),
            Self::Embeddings => Some("embeddings"),
            Self::StructuredOutput => Some("structured"),
            Self::Batch => Some("batch"),
            Self::ImageGeneration => Some("image_gen"),
            Self::ComputerUse => Some("computer_use"),
            Self::McpClient | Self::McpServer => Some("mcp"),
        }
    }

    /// Check whether this capability requires a feature flag to be compiled in.
    pub fn is_feature_gated(&self) -> bool {
        self.feature_flag().is_some()
    }

    /// Get the runtime module path this capability maps to.
    pub fn module_path(&self) -> &'static str {
        match self {
            Self::Text => "core",
            Self::Streaming => "streaming",
            Self::Vision => "multimodal.vision",
            Self::Audio => "multimodal.audio",
            Self::Video => "multimodal.video",
            Self::Tools => "tools",
            Self::ParallelTools => "tools.parallel",
            Self::Agentic => "agentic",
            Self::Reasoning => "reasoning",
            Self::Embeddings => "embeddings",
            Self::StructuredOutput => "structured",
            Self::Batch => "batch",
            Self::ImageGeneration => "generation.image",
            Self::ComputerUse => "computer_use",
            Self::McpClient => "mcp.client",
            Self::McpServer => "mcp.server",
        }
    }
}

/// V2 structured capability declaration with required/optional separation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CapabilitiesV2 {
    /// V2 structured format: `{ required: [...], optional: [...], feature_flags: {...} }`
    Structured {
        required: Vec<Capability>,
        #[serde(default)]
        optional: Vec<Capability>,
        #[serde(default)]
        feature_flags: FeatureFlags,
    },
    /// V1 legacy flat format: `{ streaming: true, tools: true, vision: false }`
    Legacy(LegacyCapabilities),
}

impl CapabilitiesV2 {
    /// Get all capabilities (required + optional) as a unified set.
    pub fn all_capabilities(&self) -> Vec<Capability> {
        match self {
            Self::Structured { required, optional, .. } => {
                let mut all = required.clone();
                all.extend(optional.iter().cloned());
                all
            }
            Self::Legacy(legacy) => legacy.to_capabilities(),
        }
    }

    /// Get only the required capabilities.
    pub fn required_capabilities(&self) -> Vec<Capability> {
        match self {
            Self::Structured { required, .. } => required.clone(),
            Self::Legacy(legacy) => {
                // V1 legacy: text is always required, streaming if declared
                let mut req = vec![Capability::Text];
                if legacy.streaming {
                    req.push(Capability::Streaming);
                }
                req
            }
        }
    }

    /// Check if a specific capability is declared (required or optional).
    pub fn has_capability(&self, cap: Capability) -> bool {
        self.all_capabilities().contains(&cap)
    }

    /// Get the feature flags.
    pub fn feature_flags(&self) -> FeatureFlags {
        match self {
            Self::Structured { feature_flags, .. } => feature_flags.clone(),
            Self::Legacy(_) => FeatureFlags::default(),
        }
    }

    /// Auto-promote V1 legacy capabilities to V2 structured format.
    pub fn promote_to_v2(&self) -> Self {
        match self {
            Self::Structured { .. } => self.clone(),
            Self::Legacy(legacy) => {
                let mut required = vec![Capability::Text];
                let mut optional = Vec::new();

                if legacy.streaming {
                    required.push(Capability::Streaming);
                }
                if legacy.tools {
                    optional.push(Capability::Tools);
                }
                if legacy.vision {
                    optional.push(Capability::Vision);
                }
                if legacy.agentic {
                    optional.push(Capability::Agentic);
                }
                if legacy.reasoning {
                    optional.push(Capability::Reasoning);
                }
                if legacy.parallel_tools {
                    optional.push(Capability::ParallelTools);
                }

                Self::Structured {
                    required,
                    optional,
                    feature_flags: FeatureFlags::default(),
                }
            }
        }
    }
}

/// V1 legacy boolean capability flags — backward compatible.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyCapabilities {
    #[serde(default)]
    pub streaming: bool,
    #[serde(default)]
    pub tools: bool,
    #[serde(default)]
    pub vision: bool,
    #[serde(default)]
    pub agentic: bool,
    #[serde(default)]
    pub reasoning: bool,
    #[serde(default)]
    pub parallel_tools: bool,
}

impl LegacyCapabilities {
    fn to_capabilities(&self) -> Vec<Capability> {
        let mut caps = vec![Capability::Text];
        if self.streaming { caps.push(Capability::Streaming); }
        if self.tools { caps.push(Capability::Tools); }
        if self.vision { caps.push(Capability::Vision); }
        if self.agentic { caps.push(Capability::Agentic); }
        if self.reasoning { caps.push(Capability::Reasoning); }
        if self.parallel_tools { caps.push(Capability::ParallelTools); }
        caps
    }
}

/// Fine-grained feature toggles within capabilities.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FeatureFlags {
    #[serde(default)]
    pub structured_output: bool,
    #[serde(default)]
    pub parallel_tool_calls: bool,
    #[serde(default)]
    pub extended_thinking: bool,
    #[serde(default)]
    pub streaming_usage: bool,
    #[serde(default)]
    pub system_messages: bool,
    #[serde(default)]
    pub image_generation: bool,
    /// Additional provider-specific flags.
    #[serde(flatten)]
    pub extra: HashMap<String, bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_feature_flags() {
        assert_eq!(Capability::Text.feature_flag(), None);
        assert_eq!(Capability::McpClient.feature_flag(), Some("mcp"));
        assert_eq!(Capability::ComputerUse.feature_flag(), Some("computer_use"));
        assert!(!Capability::Streaming.is_feature_gated());
        assert!(Capability::Audio.is_feature_gated());
    }

    #[test]
    fn test_v2_capabilities_structured() {
        let json = r#"{
            "required": ["text", "streaming", "tools"],
            "optional": ["vision", "mcp_client"],
            "feature_flags": {"structured_output": true}
        }"#;
        let caps: CapabilitiesV2 = serde_json::from_str(json).unwrap();
        assert!(caps.has_capability(Capability::Text));
        assert!(caps.has_capability(Capability::McpClient));
        assert!(!caps.has_capability(Capability::ComputerUse));
        assert!(caps.feature_flags().structured_output);
    }

    #[test]
    fn test_legacy_promotion() {
        let legacy = LegacyCapabilities {
            streaming: true,
            tools: true,
            vision: true,
            agentic: false,
            reasoning: false,
            parallel_tools: false,
        };
        let v1 = CapabilitiesV2::Legacy(legacy);
        let v2 = v1.promote_to_v2();
        match &v2 {
            CapabilitiesV2::Structured { required, optional, .. } => {
                assert!(required.contains(&Capability::Text));
                assert!(required.contains(&Capability::Streaming));
                assert!(optional.contains(&Capability::Tools));
                assert!(optional.contains(&Capability::Vision));
            }
            _ => panic!("Expected Structured"),
        }
    }
}
