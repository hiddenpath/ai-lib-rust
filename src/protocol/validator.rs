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
    /// 1. GitHub URL (canonical source) - priority, used in production and CI
    /// 2. AI_PROTOCOL_DIR as GitHub URL (if set and is a URL)
    /// 3. Local file system (for offline development) - fallback if GitHub unavailable
    ///
    /// This ensures all validation uses the same standard schema, while allowing
    /// local development when network is unavailable.
    pub fn new() -> Result<Self, ProtocolError> {
        // Priority 1: Try local file system first (for development)
        // This allows developers to test schema changes locally before pushing to GitHub
        let schema_content = Self::load_schema_from_local()
            .or_else(|| {
                // Priority 2: Try GitHub URL (canonical source)
                Self::fetch_schema_from_github().ok()
            })
            .or_else(|| {
                // Priority 3: Try AI_PROTOCOL_DIR as GitHub URL (if it's a URL)
                if let Ok(root) =
                    std::env::var("AI_PROTOCOL_DIR").or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
                {
                    if root.starts_with("http://") || root.starts_with("https://") {
                        let schema_url = if root.ends_with('/') {
                            format!("{}schemas/v1.json", root)
                        } else {
                            format!("{}/schemas/v1.json", root)
                        };
                        Self::fetch_schema_from_url(&schema_url).ok()
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .or_else(|| {
                // Priority 4: Embedded canonical schema (offline-safe for published crates).
                Some(Self::embedded_schema_v1().to_string())
            })
            .unwrap_or_else(|| {
                // Final fallback (offline-safe): use a minimal built-in schema so the runtime
                // can still operate, and rely on basic validation + runtime checks.
                tracing::warn!(
                    "AI-Protocol JSON Schema not found (offline). Falling back to built-in minimal schema. \
                     Tip: set AI_PROTOCOL_PATH to your local ai-protocol checkout or a GitHub raw URL."
                );
                Self::builtin_minimal_schema()
            });

        let schema_value: serde_json::Value = serde_json::from_str(&schema_content)
            .map_err(|e| ProtocolError::Internal(format!("Invalid JSON Schema: {}", e)))?;

        let schema = JSONSchema::options()
            .with_draft(Draft::Draft7)
            .compile(&schema_value)
            .map_err(|e| ProtocolError::Internal(format!("Failed to compile schema: {}", e)))?;

        Ok(Self { schema })
    }

    /// Minimal schema used as an offline fallback when the canonical schema cannot be loaded.
    ///
    /// This schema is intentionally conservative: it checks for presence of the most critical
    /// top-level fields, but does not attempt to fully validate all nested shapes.
    fn builtin_minimal_schema() -> String {
        // Draft7 is used by the `jsonschema` crate defaults we compile with.
        // We keep this small to avoid embedding large schema assets in the runtime crate.
        r#"{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "type": "object",
  "required": [
    "id",
    "protocol_version",
    "endpoint",
    "availability",
    "capabilities",
    "auth",
    "status",
    "category",
    "official_url",
    "support_contact",
    "parameter_mappings"
  ],
  "properties": {
    "id": { "type": "string", "minLength": 1 },
    "protocol_version": { "type": "string", "minLength": 1 },
    "endpoint": {
      "type": "object",
      "required": ["base_url"],
      "properties": { "base_url": { "type": "string", "minLength": 1 } }
    },
    "availability": { "type": "object" },
    "capabilities": { "type": "object" },
    "auth": { "type": "object" },
    "parameter_mappings": { "type": "object" }
  },
  "additionalProperties": true
}"#
        .to_string()
    }

    /// Embedded canonical AI-Protocol schema (v1.json) shipped with the crate.
    ///
    /// This guarantees schema validation works for published crates even when:
    /// - GitHub is unreachable
    /// - the user does not have a local ai-protocol checkout
    fn embedded_schema_v1() -> &'static str {
        include_str!("schema_v1.json")
    }

    /// Fetch schema from a specific URL.
    fn fetch_schema_from_url(
        url: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Use a separate thread to avoid tokio runtime nesting issues
        // This ensures the blocking client runs in its own thread context
        let url = url.to_string();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let result = (|| -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
                let client = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

                let response = client
                    .get(&url)
                    .send()
                    .map_err(|e| format!("HTTP request failed: {}", e))?;

                if !response.status().is_success() {
                    return Err(format!(
                        "HTTP {}: {}",
                        response.status(),
                        response.text().unwrap_or_default()
                    )
                    .into());
                }

                Ok(response
                    .text()
                    .map_err(|e| format!("Failed to read response: {}", e))?)
            })();

            let _ = tx.send(result);
        });

        rx.recv()
            .map_err(|e| format!("Failed to receive result from thread: {}", e))?
    }

    /// Fetch schema from GitHub (canonical source).
    fn fetch_schema_from_github() -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        Self::fetch_schema_from_url(Self::SCHEMA_GITHUB_URL)
    }

    /// Load schema from local file system (fallback for offline development).
    fn load_schema_from_local() -> Option<String> {
        use std::path::PathBuf;

        let mut schema_paths: Vec<PathBuf> = Vec::new();

        // If AI_PROTOCOL_DIR/AI_PROTOCOL_PATH is set and is a local path, try resolving it in a
        // few robust ways (tests often set relative paths, and the test binary cwd is not crate root).
        if let Ok(root) =
            std::env::var("AI_PROTOCOL_DIR").or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        {
            if !root.starts_with("http://") && !root.starts_with("https://") {
                let root_pb = PathBuf::from(&root);

                // Candidate bases to resolve relative paths:
                // - as-is (if absolute)
                // - relative to current_dir
                // - relative to current_exe dir
                // - relative to crate root (compile-time)
                let mut bases: Vec<PathBuf> = Vec::new();
                bases.push(root_pb.clone());
                if root_pb.is_relative() {
                    if let Ok(cd) = std::env::current_dir() {
                        bases.push(cd.join(&root_pb));
                    }
                    if let Ok(exe) = std::env::current_exe() {
                        if let Some(dir) = exe.parent() {
                            bases.push(dir.join(&root_pb));
                        }
                    }
                    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
                    bases.push(crate_dir.join(&root_pb));
                }

                for base in bases {
                    // Allow env var to point either to repo root or directly to schema file.
                    if base.extension().and_then(|s| s.to_str()) == Some("json") {
                        schema_paths.push(base.clone());
                    } else {
                        schema_paths.push(base.join("schemas").join("v1.json"));
                    }
                }
            }
        }

        // Priority 2: Windows development convenience path (always check, add if exists)
        let win_dev = PathBuf::from(r"D:\ai-protocol\schemas\v1.json");
        schema_paths.push(win_dev);

        // Priority 3: Common development paths (relative to crate root for determinism).
        let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        schema_paths.push(
            crate_dir
                .join("ai-protocol")
                .join("schemas")
                .join("v1.json"),
        );
        schema_paths.push(
            crate_dir
                .join("..")
                .join("ai-protocol")
                .join("schemas")
                .join("v1.json"),
        );
        schema_paths.push(
            crate_dir
                .join("..")
                .join("..")
                .join("ai-protocol")
                .join("schemas")
                .join("v1.json"),
        );

        // Try all paths in order
        for path in &schema_paths {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(path) {
                    return Some(content);
                }
            }
        }

        None
    }

    /// Validate a protocol manifest using the compiled JSON Schema
    pub fn validate(&self, manifest: &ProtocolManifest) -> Result<(), ProtocolError> {
        // Convert manifest to JSON for validation
        let manifest_json = serde_json::to_value(manifest)
            .map_err(|e| ProtocolError::ValidationError(format!("Serialization error: {}", e)))?;

        // 1. JSON Schema validation
        // The bundled/remote schema here is v1-focused. For v2 manifests, skip strict
        // v1 schema enforcement and rely on runtime basic checks + v2-specific paths.
        if manifest.protocol_version.starts_with("1.") {
            if let Err(errors) = self.schema.validate(&manifest_json) {
                let error_msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
                return Err(ProtocolError::ValidationError(format!(
                    "JSON Schema validation failed:\n  - {}",
                    error_msgs.join("\n  - ")
                ))
                .with_hint(
                    "Check the official AI-Protocol documentation for the required file structure.",
                ));
            }
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

        if manifest.endpoint.base_url.is_empty() {
            return Err(ProtocolError::ValidationError(
                "Base URL is required".to_string(),
            ));
        }

        // Validate protocol version
        if !(manifest.protocol_version.starts_with("1.") || manifest.protocol_version.starts_with("2."))
        {
            return Err(ProtocolError::InvalidVersion {
                version: manifest.protocol_version.clone(),
                max_supported: "2.x".to_string(),
                hint: Some(
                    "This version of the library supports AI-Protocol v1.x and v2.x manifests."
                        .to_string(),
                ),
            });
        }

        Ok(())
    }
}

impl Default for ProtocolValidator {
    fn default() -> Self {
        Self::new().expect("Failed to initialize validator")
    }
}
