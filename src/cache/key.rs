//! Cache key generation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CacheKey {
    pub hash: String,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub fingerprint: Option<String>,
}

impl CacheKey {
    pub fn new(hash: impl Into<String>) -> Self {
        Self { hash: hash.into(), model: None, provider: None, fingerprint: None }
    }
    pub fn with_model(mut self, model: impl Into<String>) -> Self { self.model = Some(model.into()); self }
    pub fn with_provider(mut self, provider: impl Into<String>) -> Self { self.provider = Some(provider.into()); self }
    pub fn with_fingerprint(mut self, fp: impl Into<String>) -> Self { self.fingerprint = Some(fp.into()); self }
    pub fn as_str(&self) -> &str { &self.hash }
}

impl std::fmt::Display for CacheKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.hash) }
}

impl From<&str> for CacheKey { fn from(s: &str) -> Self { Self::new(s) } }
impl From<String> for CacheKey { fn from(s: String) -> Self { Self::new(s) } }

pub struct CacheKeyGenerator {
    include_model: bool,
    include_temperature: bool,
    salt: Option<String>,
}

impl CacheKeyGenerator {
    pub fn new() -> Self { Self { include_model: true, include_temperature: true, salt: None } }
    pub fn with_salt(mut self, salt: impl Into<String>) -> Self { self.salt = Some(salt.into()); self }

    pub fn generate(&self, model: Option<&str>, messages: &[serde_json::Value], temperature: Option<f64>, _max_tokens: Option<u32>) -> CacheKey {
        let mut parts: BTreeMap<String, String> = BTreeMap::new();
        if self.include_model { if let Some(m) = model { parts.insert("model".into(), m.into()); } }
        if self.include_temperature { if let Some(t) = temperature { parts.insert("temperature".into(), format!("{:.2}", t)); } }
        parts.insert("messages".into(), serde_json::to_string(messages).unwrap_or_default());
        if let Some(ref s) = self.salt { parts.insert("salt".into(), s.clone()); }
        let canonical = serde_json::to_string(&parts).unwrap_or_default();
        let mut hasher = Sha256::new();
        hasher.update(canonical.as_bytes());
        let hash: String = hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect();
        let mut key = CacheKey::new(hash);
        if let Some(m) = model { key = key.with_model(m); }
        key
    }

    pub fn generate_from_json(&self, request: &serde_json::Value) -> CacheKey {
        self.generate(request["model"].as_str(), request["messages"].as_array().cloned().unwrap_or_default().as_slice(), request["temperature"].as_f64(), request["max_tokens"].as_u64().map(|v| v as u32))
    }
}

impl Default for CacheKeyGenerator { fn default() -> Self { Self::new() } }
