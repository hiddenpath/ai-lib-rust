//! Protocol validator using JSON Schema

use crate::protocol::{ProtocolError, ProtocolManifest};
use jsonschema::{Draft, JSONSchema};

/// Protocol validator that validates manifests against JSON Schema
pub struct ProtocolValidator {
    schema: JSONSchema,
}

impl ProtocolValidator {
    /// Standard GitHub URL for the official AI-Protocol schema.
    /// This is the canonical source of truth for schema validation.
    const SCHEMA_GITHUB_URL: &'static str =
        "https://raw.githubusercontent.com/hiddenpath/ai-protocol/main/schemas/v1.json";

    /// Create a new validator with the v1 schema.
    ///
    /// Schema loading strategy (in order):
    /// 1. GitHub URL (canonical source) - used in production and CI
    /// 2. Local file system (for offline development) - fallback if GitHub unavailable
    ///
    /// This ensures all validation uses the same standard schema, while allowing
    /// local development when network is unavailable.
    pub fn new() -> Result<Self, ProtocolError> {
        // Try GitHub URL first (canonical source)
        let schema_content = if let Ok(content) = Self::fetch_schema_from_github() {
            Some(content)
        } else {
            // Fallback to local file system for offline development
            Self::load_schema_from_local()
        }
        .ok_or_else(|| {
            ProtocolError::SchemaError(
                "JSON Schema not found: GitHub URL unavailable and no local file found. \
                 Set AI_PROTOCOL_DIR for local development, or ensure network access for GitHub."
                    .to_string(),
            )
        })?;

        let schema_value: serde_json::Value = serde_json::from_str(&schema_content)
            .map_err(|e| ProtocolError::SchemaError(format!("Invalid JSON Schema: {}", e)))?;

        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_value)
            .map_err(|e| ProtocolError::SchemaError(format!("Failed to compile schema: {}", e)))?;

        Ok(Self { schema })
    }

    /// Fetch schema from GitHub (canonical source).
    fn fetch_schema_from_github() -> Result<String, Box<dyn std::error::Error>> {
        // Use tokio runtime to execute async HTTP request
        // This is acceptable since validator is typically created at startup
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| format!("Failed to create tokio runtime: {}", e))?;
        
        rt.block_on(async {
            let client = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()
                .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
            
            let response = client
                .get(Self::SCHEMA_GITHUB_URL)
                .send()
                .await
                .map_err(|e| format!("HTTP request failed: {}", e))?;
            
            if !response.status().is_success() {
                return Err(format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                )
                .into());
            }
            
            Ok(response.text().await.map_err(|e| format!("Failed to read response: {}", e))?)
        })
    }

    /// Load schema from local file system (fallback for offline development).
    fn load_schema_from_local() -> Option<String> {
        let mut schema_paths = vec![
            "ai-protocol/schemas/v1.json".to_string(),
            "../ai-protocol/schemas/v1.json".to_string(),
            "../../ai-protocol/schemas/v1.json".to_string(),
        ];

        if let Ok(root) =
            std::env::var("AI_PROTOCOL_DIR").or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        {
            schema_paths.insert(0, format!("{}/schemas/v1.json", root));
        }

        schema_paths
            .iter()
            .find_map(|path| std::fs::read_to_string(path).ok())
    }

    /// Validate a protocol manifest using the compiled JSON Schema
    pub fn validate(&self, manifest: &ProtocolManifest) -> Result<(), ProtocolError> {
        // Convert manifest to JSON for validation
        let manifest_json = serde_json::to_value(manifest)
            .map_err(|e| ProtocolError::ValidationError(format!("Serialization error: {}", e)))?;

        // 1. JSON Schema validation
        if let Err(errors) = self.schema.validate(&manifest_json) {
            let error_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
            return Err(ProtocolError::ValidationError(format!(
                "JSON Schema validation failed:\n  - {}",
                error_msgs.join("\n  - ")
            )));
        }

        // 2. Perform basic logic validation
        Self::validate_basic(manifest)?;

        Ok(())
    }

    /// Basic validation without JSON Schema (fallback)
    fn validate_basic(manifest: &ProtocolManifest) -> Result<(), ProtocolError> {
        // Check required fields
        if manifest.id.is_empty() {
            return Err(ProtocolError::ValidationError(
                "Protocol id is required".to_string(),
            ));
        }

        if manifest.protocol_version.is_empty() {
            return Err(ProtocolError::ValidationError(
                "Protocol version is required".to_string(),
            ));
        }

        if manifest.base_url.is_empty() {
            return Err(ProtocolError::ValidationError(
                "Base URL is required".to_string(),
            ));
        }

        // Validate protocol version
        if !manifest.protocol_version.starts_with("1.") {
            return Err(ProtocolError::InvalidVersion(format!(
                "Unsupported protocol version: {}",
                manifest.protocol_version
            )));
        }

        Ok(())
    }
}

impl Default for ProtocolValidator {
    fn default() -> Self {
        Self::new().expect("Failed to initialize validator")
    }
}
