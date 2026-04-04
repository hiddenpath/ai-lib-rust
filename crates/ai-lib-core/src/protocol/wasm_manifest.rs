//! In-memory manifest load: YAML bytes + `ProtocolValidator` (no async `ProtocolLoader` / network).

use super::{ProtocolError, ProtocolManifest, ProtocolValidator};

/// Parse and validate a provider manifest from in-memory YAML (PT-072 / WASI).
pub fn load_manifest_validated(bytes: &[u8]) -> Result<ProtocolManifest, ProtocolError> {
    let validator = ProtocolValidator::new()?;
    let manifest: ProtocolManifest = serde_yaml::from_slice(bytes).map_err(|e| {
        ProtocolError::Internal(format!("Failed to parse manifest YAML: {}", e))
    })?;
    validator.validate(&manifest)?;
    Ok(manifest)
}
