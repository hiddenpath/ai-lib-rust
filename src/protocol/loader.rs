//! Protocol loader with support for local files, embedded assets, and remote URLs
//! Heartbeat sync - 2026-01-06
//! Includes hot-reload capability using ArcSwap

use crate::protocol::{ProtocolError, ProtocolManifest};
use arc_swap::ArcSwap;
use lru::LruCache;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// Protocol loader that supports multiple sources
pub struct ProtocolLoader {
    base_path: Option<PathBuf>,
    hot_reload: bool,
    validator: crate::protocol::validator::ProtocolValidator,
    cache: Mutex<LruCache<String, Arc<ProtocolManifest>>>,
}

impl ProtocolLoader {
    pub fn new() -> Self {
        Self {
            base_path: None,
            hot_reload: false,
            validator: crate::protocol::validator::ProtocolValidator::default(),
            // Use 100 as default cache size
            // NonZeroUsize::new(100) is guaranteed to be Some, but use expect for clarity
            cache: Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(100)
                    .expect("Cache size must be non-zero (this should never happen)"),
            )),
        }
    }

    /// Set base path for protocol files
    pub fn with_base_path(mut self, path: impl AsRef<Path>) -> Self {
        self.base_path = Some(path.as_ref().to_path_buf());
        self
    }

    /// Enable hot reload
    pub fn with_hot_reload(mut self, enable: bool) -> Self {
        self.hot_reload = enable;
        self
    }

    /// Load a model configuration
    /// Model identifier format: "provider/model-name"
    pub async fn load_model(&self, model: &str) -> Result<ProtocolManifest, ProtocolError> {
        // 1. Check Cache
        {
            let mut cache = self.cache.lock().map_err(|e| {
                ProtocolError::Internal(format!(
                    "Failed to acquire cache lock while loading model '{}': {}",
                    model, e
                ))
            })?;
            if let Some(manifest) = cache.get(model) {
                return Ok(manifest.as_ref().clone());
            }
        }

        let parts: Vec<&str> = model.split('/').collect();
        if parts.len() != 2 {
            return Err(ProtocolError::NotFound {
                id: model.to_string(),
                hint: Some("Ensure the model name follows the 'provider/model' format".to_string()),
            });
        }

        let provider = parts[0];
        let model_name = parts[1];

        // First, try to load model registry to get provider reference.
        // If registry doesn't contain this model (common for providers like deepseek),
        // fall back to loading provider manifest directly using the provider segment.
        let manifest = match self.load_model_config(model_name).await {
            Ok(model_config) => self.load_provider(&model_config.provider).await?,
            Err(ProtocolError::NotFound { .. }) => self.load_provider(provider).await?,
            Err(e) => return Err(e),
        };

        // 2. Update Cache
        {
            let mut cache = self.cache.lock().map_err(|e| {
                ProtocolError::Internal(format!(
                    "Failed to acquire cache lock while caching model '{}': {}",
                    model, e
                ))
            })?;
            cache.put(model.to_string(), Arc::new(manifest.clone()));
        }

        Ok(manifest)
    }

    /// Load provider configuration
    pub async fn load_provider(
        &self,
        provider_id: &str,
    ) -> Result<ProtocolManifest, ProtocolError> {
        // Try multiple sources in order:
        // 1. Local file system (dist JSON) - PREFERRED
        // 2. Local file system (source YAML) - FALLBACK
        // 3. GitHub URL (if AI_PROTOCOL_DIR is a URL)
        // 4. Embedded assets (future)

        // Path prioritization helper
        let mut search_locations: Vec<(PathBuf, bool)> = Vec::new(); // (path_base, is_json_preferred)

        // 1. Check user-configured base_path
        if let Some(ref base_path) = self.base_path {
            // Priority 1: dist/v1/providers/{id}.json
            search_locations.push((base_path.join("dist").join("v1").join("providers"), true));
            // Priority 2: v1/providers/{id}.yaml
            search_locations.push((base_path.join("v1").join("providers"), false));
        }

        // 2. Check AI_PROTOCOL_DIR Env Var
        if let Ok(root) =
            std::env::var("AI_PROTOCOL_DIR").or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        {
            if root.starts_with("http://") || root.starts_with("https://") {
                // Handling URL sources (Remote)
                // Try JSON first if it looks like a raw github url, but typically raw github urls are specific.
                // For simplicity, we stick to the existing logic for URLs but could enhance later to try .json
                let url = if root.ends_with('/') {
                    format!("{}dist/v1/providers/{}.json", root, provider_id)
                } else {
                    format!("{}/dist/v1/providers/{}.json", root, provider_id)
                };

                // Try JSON from remote
                if let Ok(manifest) = self.load_from_json_url(&url).await {
                    return Ok(manifest);
                }

                // Fallback to YAML from remote
                let url_yaml = if root.ends_with('/') {
                    format!("{}v1/providers/{}.yaml", root, provider_id)
                } else {
                    format!("{}/v1/providers/{}.yaml", root, provider_id)
                };
                return self.load_from_url(&url_yaml).await;
            } else {
                // Local Path from Env
                let root = PathBuf::from(root);
                search_locations.push((root.join("dist").join("v1").join("providers"), true));
                search_locations.push((root.join("v1").join("providers"), false));
            }
        }

        // 3. Default dev locations
        let default_roots = vec![
            PathBuf::from("ai-protocol"),
            PathBuf::from("../ai-protocol"),
            PathBuf::from("../../ai-protocol"),
            PathBuf::from("D:\\ai-protocol"),
        ];

        for root in default_roots {
            search_locations.push((root.join("dist").join("v1").join("providers"), true));
            search_locations.push((root.join("v1").join("providers"), false));
        }

        // Execute Search
        for (base, prefer_json) in search_locations {
            if prefer_json {
                let json_path = base.join(format!("{}.json", provider_id));
                if json_path.exists() {
                    return self.load_from_json_file(&json_path).await;
                }
            } else {
                let yaml_path = base.join(format!("{}.yaml", provider_id));
                if yaml_path.exists() {
                    return self.load_from_file(&yaml_path).await;
                }
            }
        }

        // Last resort: try GitHub raw URL (canonical source) - JSON
        let github_json = format!(
            "https://raw.githubusercontent.com/hiddenpath/ai-protocol/main/dist/v1/providers/{}.json",
            provider_id
        );
        if let Ok(manifest) = self.load_from_json_url(&github_json).await {
            return Ok(manifest);
        }

        // Last resort fallback: YAML
        let github_yaml = format!(
            "https://raw.githubusercontent.com/hiddenpath/ai-protocol/main/v1/providers/{}.yaml",
            provider_id
        );
        if let Ok(manifest) = self.load_from_url(&github_yaml).await {
            return Ok(manifest);
        }

        Err(ProtocolError::NotFound {
            id: provider_id.to_string(),
            hint: Some(format!(
                "Check if the provider file '{}.json' or '{}.yaml' exists in your protocol directory",
                provider_id, provider_id
            )),
        })
    }

    /// Load protocol from local JSON file (Fast Path)
    async fn load_from_json_file(&self, path: &Path) -> Result<ProtocolManifest, ProtocolError> {
        let content = tokio::fs::read(path)
            .await
            .map_err(|e| ProtocolError::LoadError {
                path: path.to_string_lossy().to_string(),
                reason: e.to_string(),
                hint: Some("Check file permissions.".to_string()),
            })?;

        let manifest: ProtocolManifest = serde_json::from_slice(&content)
            .map_err(|e| ProtocolError::ValidationError(format!("Invalid JSON manifest: {}", e)))?;

        // Validate against JSON Schema (Optional but recommended even for dist)
        // For max speed, we might skip this in release, but keeping for safety now.
        self.validator.validate(&manifest)?;

        Ok(manifest)
    }

    /// Load protocol from local YAML file (Legacy/Dev Path)
    async fn load_from_file(&self, path: &Path) -> Result<ProtocolManifest, ProtocolError> {
        // Read as bytes first to handle different encodings
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| ProtocolError::LoadError {
                path: path.to_string_lossy().to_string(),
                reason: e.to_string(),
                hint: Some("Check if the file exists and you have read permissions.".to_string()),
            })?;

        // ... (encoding detection remains same)
        let content = if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
            // UTF-16 LE with BOM
            let utf16_bytes = &bytes[2..];
            let mut utf16_chars = Vec::new();
            for i in (0..utf16_bytes.len()).step_by(2) {
                if i + 1 < utf16_bytes.len() {
                    let code_unit = u16::from_le_bytes([utf16_bytes[i], utf16_bytes[i + 1]]);
                    utf16_chars.push(code_unit);
                }
            }
            String::from_utf16(&utf16_chars).map_err(|e| ProtocolError::LoadError {
                path: path.to_string_lossy().to_string(),
                reason: format!("Invalid UTF-16: {}", e),
                hint: None,
            })?
        } else if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
            String::from_utf8(bytes[3..].to_vec()).map_err(|e| ProtocolError::LoadError {
                path: path.to_string_lossy().to_string(),
                reason: format!("Invalid UTF-8 (after BOM): {}", e),
                hint: None,
            })?
        } else {
            String::from_utf8(bytes).map_err(|e| ProtocolError::LoadError {
                path: path.to_string_lossy().to_string(),
                reason: format!("Invalid UTF-8: {}", e),
                hint: None,
            })?
        };

        let manifest: ProtocolManifest = Self::parse_manifest_yaml(&content)?;
        self.validator.validate(&manifest)?;
        Ok(manifest)
    }

    /// Load protocol from remote JSON URL
    async fn load_from_json_url(&self, url: &str) -> Result<ProtocolManifest, ProtocolError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProtocolError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| ProtocolError::LoadError {
                path: url.to_string(),
                reason: format!("HTTP request failed: {}", e),
                hint: None,
            })?;

        if !response.status().is_success() {
            return Err(ProtocolError::LoadError {
                path: url.to_string(),
                reason: format!("HTTP {}", response.status()),
                hint: None,
            });
        }

        let content = response
            .bytes()
            .await
            .map_err(|e| ProtocolError::LoadError {
                path: url.to_string(),
                reason: format!("Failed to read bytes: {}", e),
                hint: None,
            })?;

        let manifest: ProtocolManifest = serde_json::from_slice(&content).map_err(|e| {
            ProtocolError::ValidationError(format!("Invalid JSON manifest from URL: {}", e))
        })?;

        self.validator.validate(&manifest)?;
        Ok(manifest)
    }

    /// Load protocol from remote URL (GitHub raw URL)
    async fn load_from_url(&self, url: &str) -> Result<ProtocolManifest, ProtocolError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProtocolError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| ProtocolError::LoadError {
                path: url.to_string(),
                reason: format!("HTTP request failed: {}", e),
                hint: Some(
                    "Check your internet connection and verify the URL is accessible.".to_string(),
                ),
            })?;

        if !response.status().is_success() {
            return Err(ProtocolError::LoadError {
                path: url.to_string(),
                reason: format!(
                    "HTTP {}: {}",
                    response.status(),
                    response.text().await.unwrap_or_default()
                ),
                hint: Some(
                    "Verify the remote registry URL and your API permissions if any.".to_string(),
                ),
            });
        }

        let content = response
            .text()
            .await
            .map_err(|e| ProtocolError::LoadError {
                path: url.to_string(),
                reason: format!("Failed to read response: {}", e),
                hint: None,
            })?;

        let manifest: ProtocolManifest = Self::parse_manifest_yaml(&content)?;

        // Validate against JSON Schema
        self.validator.validate(&manifest)?;

        Ok(manifest)
    }

    /// Parse YAML into a ProtocolManifest with better error classification.
    ///
    /// Rationale:
    /// - YAML syntax/encoding issues are "load" errors.
    /// - Structural mismatches (missing required fields, wrong types) are "validation" errors.
    fn parse_manifest_yaml(content: &str) -> Result<ProtocolManifest, ProtocolError> {
        serde_yaml::from_str::<ProtocolManifest>(content).map_err(|e| {
            let msg = e.to_string();
            // Heuristic classification based on serde error messages.
            // This keeps public error categories stable without pulling in serde internals.
            let looks_structural = msg.contains("missing field")
                || msg.contains("unknown field")
                || msg.contains("invalid type")
                || msg.contains("invalid value")
                || msg.contains("expected");

            if looks_structural {
                ProtocolError::ValidationError(format!("Invalid manifest structure: {}", msg))
            } else {
                ProtocolError::YamlError(msg)
            }
        })
    }

    /// Load model configuration from registry
    async fn load_model_config(&self, model_name: &str) -> Result<ModelConfig, ProtocolError> {
        // Try to find model, scanning registries.
        // Priority: dist/v1/models/*.json -> v1/models/*.yaml

        let mut search_locations: Vec<(PathBuf, bool)> = Vec::new(); // (path_base, is_json_preferred)

        // 1. Env Var AI_PROTOCOL_DIR
        if let Ok(root) =
            std::env::var("AI_PROTOCOL_DIR").or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        {
            // If HTTP, skipped here as typically model config loading implies local or full repo clone access.
            // If we really need remote model loading, we'd need a different strategy (scanning a remote index).
            // For now, assume local model registry for this heuristic.
            if !root.starts_with("http://") && !root.starts_with("https://") {
                let root = PathBuf::from(root);
                search_locations.push((root.join("dist").join("v1").join("models"), true));
                search_locations.push((root.join("v1").join("models"), false));
            }
        }

        // 2. Default paths
        let default_roots = vec![
            PathBuf::from("ai-protocol"),
            PathBuf::from("../ai-protocol"),
            PathBuf::from("../../ai-protocol"),
            PathBuf::from("D:\\ai-protocol"),
        ];

        for root in default_roots {
            search_locations.push((root.join("dist").join("v1").join("models"), true));
            search_locations.push((root.join("v1").join("models"), false));
        }

        for (base, prefer_json) in search_locations {
            if !base.exists() {
                continue;
            }
            let mut rd = match tokio::fs::read_dir(&base).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };

            while let Ok(Some(entry)) = rd.next_entry().await {
                let path = entry.path();
                let extension = path.extension().and_then(|s| s.to_str());

                let is_match = if prefer_json {
                    extension.map(|s| s.eq_ignore_ascii_case("json")) == Some(true)
                } else {
                    extension
                        .map(|s| s.eq_ignore_ascii_case("yaml") || s.eq_ignore_ascii_case("yml"))
                        == Some(true)
                };

                if !is_match {
                    continue;
                }

                if prefer_json {
                    if let Ok(config) = self.load_model_registry_json(&path).await {
                        if let Some(model) = config.models.get(model_name) {
                            return Ok(model.clone());
                        }
                    }
                } else {
                    if let Ok(config) = self.load_model_registry_yaml(&path).await {
                        if let Some(model) = config.models.get(model_name) {
                            return Ok(model.clone());
                        }
                    }
                }
            }
        }

        Err(ProtocolError::NotFound {
            id: model_name.to_string(),
            hint: Some(
                "Check if the model is registered in the manifests/v1/models/ directory"
                    .to_string(),
            ),
        })
    }

    async fn load_model_registry_json(&self, path: &Path) -> Result<ModelRegistry, ProtocolError> {
        let content = tokio::fs::read(path)
            .await
            .map_err(|e| ProtocolError::LoadError {
                path: path.to_string_lossy().to_string(),
                reason: e.to_string(),
                hint: None,
            })?;
        let registry: ModelRegistry = serde_json::from_slice(&content).map_err(|e| {
            ProtocolError::ValidationError(format!("Invalid JSON model registry: {}", e))
        })?;
        Ok(registry)
    }

    async fn load_model_registry_yaml(&self, path: &Path) -> Result<ModelRegistry, ProtocolError> {
        let content =
            tokio::fs::read_to_string(path)
                .await
                .map_err(|e| ProtocolError::LoadError {
                    path: path.to_string_lossy().to_string(),
                    reason: format!("Failed to read model registry: {}", e),
                    hint: None,
                })?;

        let registry: ModelRegistry = serde_yaml::from_str(&content).map_err(|e| {
            ProtocolError::YamlError(format!("Failed to parse model registry: {}", e))
        })?;

        Ok(registry)
    }
}

impl Default for ProtocolLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Model registry structure
#[derive(Debug, Clone, serde::Deserialize)]
struct ModelRegistry {
    models: std::collections::HashMap<String, ModelConfig>,
}

/// Model configuration from registry
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Deserialize)]
struct ModelConfig {
    provider: String,
    #[serde(default)]
    model_id: Option<String>,
    #[serde(default)]
    context_window: Option<u32>,
    #[serde(default)]
    capabilities: Vec<String>,
}

/// Hot-reloadable protocol registry
pub struct ProtocolRegistry {
    manifests: ArcSwap<std::collections::HashMap<String, Arc<ProtocolManifest>>>,
    loader: ProtocolLoader,
}

impl ProtocolRegistry {
    pub fn new() -> Self {
        Self {
            manifests: ArcSwap::from_pointee(std::collections::HashMap::new()),
            loader: ProtocolLoader::new(),
        }
    }

    /// Get or load a protocol manifest
    pub async fn get_manifest(
        &self,
        provider_id: &str,
    ) -> Result<Arc<ProtocolManifest>, ProtocolError> {
        // Check cache first
        let current = self.manifests.load();
        if let Some(manifest) = current.get(provider_id) {
            return Ok(Arc::clone(manifest));
        }

        // Load and cache
        let manifest = self.loader.load_provider(provider_id).await?;
        let manifest_arc = Arc::new(manifest);

        // Update cache atomically
        let mut updated_map = std::collections::HashMap::new();
        for (k, v) in current.iter() {
            updated_map.insert(k.clone(), v.clone());
        }
        updated_map.insert(provider_id.to_string(), manifest_arc.clone());
        self.manifests.store(Arc::new(updated_map));

        Ok(manifest_arc)
    }
}

impl Default for ProtocolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
