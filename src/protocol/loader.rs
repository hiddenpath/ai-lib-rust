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
            cache: Mutex::new(LruCache::new(std::num::NonZeroUsize::new(100).unwrap())),
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
            let mut cache = self.cache.lock().unwrap();
            if let Some(manifest) = cache.get(model) {
                return Ok(manifest.as_ref().clone());
            }
        }

        let parts: Vec<&str> = model.split('/').collect();
        if parts.len() != 2 {
            return Err(ProtocolError::NotFound(format!(
                "Invalid model format: {}. Expected 'provider/model-name'",
                model
            )));
        }

        let provider = parts[0];
        let model_name = parts[1];

        // First, try to load model registry to get provider reference.
        // If registry doesn't contain this model (common for providers like deepseek),
        // fall back to loading provider manifest directly using the provider segment.
        let manifest = match self.load_model_config(model_name).await {
            Ok(model_config) => self.load_provider(&model_config.provider).await?,
            Err(ProtocolError::NotFound(_)) => self.load_provider(provider).await?,
            Err(e) => return Err(e),
        };

        // 2. Update Cache
        {
            let mut cache = self.cache.lock().unwrap();
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
        // 1. Local file system (if base_path is set)
        // 2. GitHub URL (if AI_PROTOCOL_DIR is a URL)
        // 3. Local file system (default paths)
        // 4. Embedded assets (future: compile-time inclusion)

        if let Some(ref base_path) = self.base_path {
            let provider_path = base_path
                .join("v1")
                .join("providers")
                .join(format!("{}.yaml", provider_id));

            if provider_path.exists() {
                return self.load_from_file(&provider_path).await;
            }
        }

        // Check if AI_PROTOCOL_DIR is a GitHub URL
        if let Ok(root) =
            std::env::var("AI_PROTOCOL_DIR").or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        {
            // Check if it's a URL (starts with http:// or https://)
            if root.starts_with("http://") || root.starts_with("https://") {
                // Construct GitHub raw URL for provider manifest
                let url = if root.ends_with('/') {
                    format!("{}v1/providers/{}.yaml", root, provider_id)
                } else {
                    format!("{}/v1/providers/{}.yaml", root, provider_id)
                };
                return self.load_from_url(&url).await;
            }
        }

        // Default search paths (local file system):
        // - env `AI_PROTOCOL_DIR` / `AI_PROTOCOL_PATH` pointing to the ai-protocol repo root
        // - relative paths for submodule/sibling setups
        // - (dev convenience) `D:\ai-protocol\...` if present
        let mut default_paths: Vec<PathBuf> = Vec::new();
        if let Ok(root) =
            std::env::var("AI_PROTOCOL_DIR").or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        {
            // Only add if it's not a URL (already handled above)
            if !root.starts_with("http://") && !root.starts_with("https://") {
                let root = PathBuf::from(root);
                default_paths.push(root.join("v1").join("providers"));
            }
        }
        default_paths.push(PathBuf::from("ai-protocol/v1/providers"));
        default_paths.push(PathBuf::from("../ai-protocol/v1/providers"));
        default_paths.push(PathBuf::from("../../ai-protocol/v1/providers"));
        let win_dev = PathBuf::from("D:\\ai-protocol\\v1\\providers");
        if win_dev.exists() {
            default_paths.push(win_dev);
        }

        for base in default_paths {
            let provider_path = base.join(format!("{}.yaml", provider_id));
            if provider_path.exists() {
                return self.load_from_file(&provider_path).await;
            }
        }

        // Last resort: try GitHub raw URL (canonical source)
        let github_url = format!(
            "https://raw.githubusercontent.com/hiddenpath/ai-protocol/main/v1/providers/{}.yaml",
            provider_id
        );
        if let Ok(manifest) = self.load_from_url(&github_url).await {
            return Ok(manifest);
        }

        Err(ProtocolError::NotFound(format!(
            "Provider configuration not found: {}",
            provider_id
        )))
    }

    /// Load protocol from local file
    async fn load_from_file(&self, path: &Path) -> Result<ProtocolManifest, ProtocolError> {
        // Read as bytes first to handle different encodings
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|e| {
                let path_str = path.to_string_lossy();
                ProtocolError::LoadError(format!("Failed to read file '{}': {}", path_str, e))
            })?;
        
        // Detect encoding and convert to UTF-8 string
        let content = if bytes.len() >= 2 && bytes[0] == 0xFF && bytes[1] == 0xFE {
            // UTF-16 LE with BOM
            let utf16_bytes = &bytes[2..];
            // Convert UTF-16 LE bytes to u16 array
            let mut utf16_chars = Vec::new();
            for i in (0..utf16_bytes.len()).step_by(2) {
                if i + 1 < utf16_bytes.len() {
                    let code_unit = u16::from_le_bytes([utf16_bytes[i], utf16_bytes[i + 1]]);
                    utf16_chars.push(code_unit);
                }
            }
            String::from_utf16(&utf16_chars)
                .map_err(|e| {
                    let path_str = path.to_string_lossy();
                    ProtocolError::LoadError(format!(
                        "File '{}' contains invalid UTF-16: {}. Please convert the file to UTF-8 encoding.",
                        path_str, e
                    ))
                })?
        } else if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
            // UTF-8 with BOM, skip BOM
            String::from_utf8(bytes[3..].to_vec())
                .map_err(|e| {
                    let path_str = path.to_string_lossy();
                    ProtocolError::LoadError(format!(
                        "File '{}' contains invalid UTF-8 (after BOM): {}",
                        path_str, e
                    ))
                })?
        } else {
            // Regular UTF-8 (no BOM)
            String::from_utf8(bytes)
                .map_err(|e| {
                    let path_str = path.to_string_lossy();
                    ProtocolError::LoadError(format!(
                        "File '{}' contains invalid UTF-8: {}. Please convert the file to UTF-8 encoding.",
                        path_str, e
                    ))
                })?
        };

        let manifest: ProtocolManifest = Self::parse_manifest_yaml(&content)?;

        // Validate against JSON Schema
        self.validator.validate(&manifest)?;

        Ok(manifest)
    }

    /// Load protocol from remote URL (GitHub raw URL)
    async fn load_from_url(&self, url: &str) -> Result<ProtocolManifest, ProtocolError> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ProtocolError::LoadError(format!("Failed to create HTTP client: {}", e)))?;

        let response = client
            .get(url)
            .send()
            .await
            .map_err(|e| ProtocolError::LoadError(format!("HTTP request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(ProtocolError::LoadError(format!(
                "HTTP {}: {}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        let content = response
            .text()
            .await
            .map_err(|e| ProtocolError::LoadError(format!("Failed to read response: {}", e)))?;

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
                ProtocolError::LoadError(format!("Failed to parse YAML: {}", msg))
            }
        })
    }

    /// Load model configuration from registry
    async fn load_model_config(&self, model_name: &str) -> Result<ModelConfig, ProtocolError> {
        // Try to find model in v1/models/ directory, scanning all `*.yaml` registries.
        let mut model_paths: Vec<PathBuf> = Vec::new();
        if let Ok(root) =
            std::env::var("AI_PROTOCOL_DIR").or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        {
            let root = PathBuf::from(root);
            model_paths.push(root.join("v1").join("models"));
        }
        model_paths.push(PathBuf::from("ai-protocol/v1/models"));
        model_paths.push(PathBuf::from("../ai-protocol/v1/models"));
        model_paths.push(PathBuf::from("../../ai-protocol/v1/models"));
        let win_dev = PathBuf::from("D:\\ai-protocol\\v1\\models");
        if win_dev.exists() {
            model_paths.push(win_dev);
        }

        for base in model_paths {
            if !base.exists() {
                continue;
            }
            let mut rd = match tokio::fs::read_dir(&base).await {
                Ok(rd) => rd,
                Err(_) => continue,
            };
            while let Ok(Some(entry)) = rd.next_entry().await {
                let path = entry.path();
                if path
                    .extension()
                    .and_then(|s| s.to_str())
                    .map(|s| s.eq_ignore_ascii_case("yaml") || s.eq_ignore_ascii_case("yml"))
                    != Some(true)
                {
                    continue;
                }
                if let Ok(config) = self.load_model_registry(&path).await {
                    if let Some(model) = config.models.get(model_name) {
                        return Ok(model.clone());
                    }
                }
            }
        }

        Err(ProtocolError::NotFound(format!(
            "Model not found: {}",
            model_name
        )))
    }

    async fn load_model_registry(&self, path: &Path) -> Result<ModelRegistry, ProtocolError> {
        let content = tokio::fs::read_to_string(path).await.map_err(|e| {
            ProtocolError::LoadError(format!("Failed to read model registry: {}", e))
        })?;

        let registry: ModelRegistry = serde_yaml::from_str(&content).map_err(|e| {
            ProtocolError::LoadError(format!("Failed to parse model registry: {}", e))
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
