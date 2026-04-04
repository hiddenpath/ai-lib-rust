//! 能力注册表 — 根据 Manifest 声明动态加载和管理运行时模块
//!
//! Capability registry that dynamically loads runtime modules based on
//! V2 Manifest capability declarations. Provides compile-time feature gate
//! checking and runtime module availability tracking.

use std::collections::{HashMap, HashSet};

use crate::protocol::v2::capabilities::Capability;

/// Runtime capability registry — tracks which capabilities are available
/// and provides module resolution based on manifest declarations.
#[derive(Debug, Clone)]
pub struct CapabilityRegistry {
    /// Capabilities declared as required by the manifest.
    required: HashSet<Capability>,
    /// Capabilities declared as optional by the manifest.
    optional: HashSet<Capability>,
    /// Capabilities that are actually available in this build (feature-gated).
    available: HashSet<Capability>,
}

impl CapabilityRegistry {
    /// Create a new registry from V2 capabilities declaration.
    pub fn from_capabilities(caps: &crate::protocol::v2::capabilities::CapabilitiesV2) -> Self {
        let required: HashSet<_> = caps.required_capabilities().into_iter().collect();
        let all: HashSet<_> = caps.all_capabilities().into_iter().collect();
        let optional: HashSet<_> = all.difference(&required).cloned().collect();

        let available = Self::detect_available_capabilities();

        Self {
            required,
            optional,
            available,
        }
    }

    /// Detect which capabilities are compiled into this build.
    fn detect_available_capabilities() -> HashSet<Capability> {
        let mut caps = HashSet::new();

        // Core capabilities — always available
        caps.insert(Capability::Text);
        caps.insert(Capability::Streaming);
        caps.insert(Capability::Tools);
        caps.insert(Capability::ParallelTools);

        // Feature-gated capabilities
        #[cfg(feature = "embeddings")]
        caps.insert(Capability::Embeddings);

        #[cfg(feature = "batch")]
        caps.insert(Capability::Batch);

        #[cfg(feature = "mcp")]
        caps.insert(Capability::McpClient);

        #[cfg(feature = "mcp")]
        caps.insert(Capability::McpServer);

        #[cfg(feature = "computer_use")]
        caps.insert(Capability::ComputerUse);

        #[cfg(feature = "multimodal")]
        {
            caps.insert(Capability::Audio);
            caps.insert(Capability::Video);
            caps.insert(Capability::Vision);
        }

        #[cfg(feature = "reasoning")]
        caps.insert(Capability::Reasoning);

        // Vision is also available without multimodal flag for basic image support
        #[cfg(not(feature = "multimodal"))]
        caps.insert(Capability::Vision);

        caps.insert(Capability::Agentic);
        caps.insert(Capability::StructuredOutput);

        caps
    }

    /// Check if a required capability is missing from this build.
    pub fn validate_requirements(&self) -> Result<(), Vec<CapabilityGap>> {
        let mut gaps = Vec::new();

        for cap in &self.required {
            if !self.available.contains(cap) {
                gaps.push(CapabilityGap {
                    capability: *cap,
                    required: true,
                    feature_flag: cap.feature_flag().map(String::from),
                });
            }
        }

        if gaps.is_empty() {
            Ok(())
        } else {
            Err(gaps)
        }
    }

    /// Get the set of capabilities that can be used (declared AND available).
    pub fn active_capabilities(&self) -> HashSet<Capability> {
        let declared: HashSet<_> = self.required.union(&self.optional).cloned().collect();
        declared.intersection(&self.available).cloned().collect()
    }

    /// Check if a specific capability is usable (declared and compiled in).
    pub fn is_active(&self, cap: Capability) -> bool {
        (self.required.contains(&cap) || self.optional.contains(&cap))
            && self.available.contains(&cap)
    }

    /// Get a human-readable status report of all capabilities.
    pub fn status_report(&self) -> HashMap<Capability, CapabilityStatus> {
        let mut report = HashMap::new();
        let all_declared: HashSet<_> = self.required.union(&self.optional).cloned().collect();

        for cap in &all_declared {
            let status = if self.available.contains(cap) {
                if self.required.contains(cap) {
                    CapabilityStatus::ActiveRequired
                } else {
                    CapabilityStatus::ActiveOptional
                }
            } else {
                CapabilityStatus::Unavailable {
                    feature_flag: cap.feature_flag().map(String::from),
                }
            };
            report.insert(*cap, status);
        }

        report
    }
}

/// Describes why a capability is unavailable.
#[derive(Debug, Clone)]
pub struct CapabilityGap {
    pub capability: Capability,
    pub required: bool,
    pub feature_flag: Option<String>,
}

impl std::fmt::Display for CapabilityGap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(flag) = &self.feature_flag {
            write!(
                f,
                "Capability {:?} is required but not available. Enable with: cargo feature '{}'",
                self.capability, flag
            )
        } else {
            write!(
                f,
                "Capability {:?} is required but not available",
                self.capability
            )
        }
    }
}

/// Status of a capability in the registry.
#[derive(Debug, Clone)]
pub enum CapabilityStatus {
    ActiveRequired,
    ActiveOptional,
    Unavailable { feature_flag: Option<String> },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::v2::capabilities::{CapabilitiesV2, FeatureFlags};

    #[test]
    fn test_registry_from_capabilities() {
        let caps = CapabilitiesV2::Structured {
            required: vec![Capability::Text, Capability::Streaming],
            optional: vec![Capability::Vision, Capability::Tools],
            feature_flags: FeatureFlags::default(),
        };
        let registry = CapabilityRegistry::from_capabilities(&caps);

        assert!(registry.is_active(Capability::Text));
        assert!(registry.is_active(Capability::Streaming));
        assert!(registry.is_active(Capability::Vision));
        assert!(registry.is_active(Capability::Tools));
    }

    #[test]
    fn test_validate_requirements_pass() {
        let caps = CapabilitiesV2::Structured {
            required: vec![Capability::Text, Capability::Streaming],
            optional: vec![],
            feature_flags: FeatureFlags::default(),
        };
        let registry = CapabilityRegistry::from_capabilities(&caps);
        assert!(registry.validate_requirements().is_ok());
    }

    #[test]
    fn test_active_capabilities() {
        let caps = CapabilitiesV2::Structured {
            required: vec![Capability::Text],
            optional: vec![Capability::Vision, Capability::McpClient],
            feature_flags: FeatureFlags::default(),
        };
        let registry = CapabilityRegistry::from_capabilities(&caps);
        let active = registry.active_capabilities();
        assert!(active.contains(&Capability::Text));
        assert!(active.contains(&Capability::Vision));
    }

    #[test]
    fn test_status_report() {
        let caps = CapabilitiesV2::Structured {
            required: vec![Capability::Text],
            optional: vec![Capability::Vision],
            feature_flags: FeatureFlags::default(),
        };
        let registry = CapabilityRegistry::from_capabilities(&caps);
        let report = registry.status_report();
        assert!(matches!(
            report.get(&Capability::Text),
            Some(CapabilityStatus::ActiveRequired)
        ));
    }
}
