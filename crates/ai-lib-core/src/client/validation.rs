//! 协议清单校验：验证版本兼容性和流式配置完整性。
//!
//! Manifest validation.

use crate::protocol::ProtocolManifest;
use crate::{Error, ErrorContext, Result};

/// Runtime-supported protocol versions
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["1.1", "1.5", "2.0"];

/// Validate protocol version compatibility between runtime and manifest.
///
/// This ensures the runtime can handle the protocol version specified in the manifest.
fn validate_protocol_version(manifest: &ProtocolManifest) -> Result<()> {
    let version = &manifest.protocol_version;

    if !SUPPORTED_PROTOCOL_VERSIONS.contains(&version.as_str()) {
        return Err(Error::validation_with_context(
            format!(
                "Unsupported protocol version: {}. Runtime supports: {:?}",
                version, SUPPORTED_PROTOCOL_VERSIONS
            ),
            ErrorContext::new()
                .with_field_path("manifest.protocol_version")
                .with_source("version_validator"),
        ));
    }

    Ok(())
}

/// Validate that the manifest supports required capabilities.
///
/// When `strict_streaming` is enabled, this performs fail-fast checks for streaming config
/// completeness to avoid ambiguous runtime behavior.
pub(crate) fn validate_manifest(manifest: &ProtocolManifest, strict_streaming: bool) -> Result<()> {
    // Contract validation: Check protocol version compatibility
    validate_protocol_version(manifest)?;

    if !strict_streaming {
        return Ok(());
    }

    // If the protocol claims streaming capability, require streaming configuration.
    if manifest.supports_capability("streaming") {
        let streaming = manifest.streaming.as_ref().ok_or_else(|| {
            Error::validation_with_context(
                "strict_streaming: manifest.streaming is required",
                ErrorContext::new()
                    .with_field_path("manifest.streaming")
                    .with_source("manifest_validator"),
            )
        })?;

        let decoder = streaming.decoder.as_ref().ok_or_else(|| {
            Error::validation_with_context(
                "strict_streaming: streaming.decoder is required",
                ErrorContext::new()
                    .with_field_path("manifest.streaming.decoder")
                    .with_source("manifest_validator"),
            )
        })?;
        if decoder.format.trim().is_empty() {
            return Err(Error::validation_with_context(
                "strict_streaming: streaming.decoder.format must be non-empty",
                ErrorContext::new()
                    .with_field_path("manifest.streaming.decoder.format")
                    .with_source("manifest_validator"),
            ));
        }

        // If no explicit event_map rules are provided, the default PathEventMapper needs paths.
        if streaming.event_map.is_empty() {
            if streaming
                .content_path
                .as_deref()
                .map(|s: &str| s.trim().is_empty())
                .unwrap_or(true)
            {
                return Err(Error::validation_with_context(
                    "strict_streaming: streaming.content_path is required when streaming.event_map is empty",
                    ErrorContext::new()
                        .with_field_path("manifest.streaming.content_path")
                        .with_source("manifest_validator"),
                ));
            }

            if manifest.supports_capability("tools")
                && streaming
                    .tool_call_path
                    .as_deref()
                    .map(|s: &str| s.trim().is_empty())
                    .unwrap_or(true)
            {
                return Err(Error::validation_with_context(
                    "strict_streaming: streaming.tool_call_path is required for tools when streaming.event_map is empty",
                    ErrorContext::new()
                        .with_field_path("manifest.streaming.tool_call_path")
                        .with_source("manifest_validator"),
                ));
            }
        }
    }

    Ok(())
}
