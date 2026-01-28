//! Cache manager.

use serde::{de::DeserializeOwned, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use super::backend::CacheBackend;
use super::key::CacheKey;
use crate::Result;

#[derive(Debug, Clone)]
pub struct CacheConfig { pub default_ttl: Duration, pub enabled: bool, pub max_entry_size: usize, pub key_prefix: Option<String> }

impl Default for CacheConfig {
    fn default() -> Self { Self { default_ttl: Duration::from_secs(3600), enabled: true, max_entry_size: 10 * 1024 * 1024, key_prefix: None } }
}

impl CacheConfig {
    pub fn new() -> Self { Self::default() }
    pub fn with_ttl(mut self, ttl: Duration) -> Self { self.default_ttl = ttl; self }
    pub fn with_enabled(mut self, enabled: bool) -> Self { self.enabled = enabled; self }
    pub fn with_key_prefix(mut self, prefix: impl Into<String>) -> Self { self.key_prefix = Some(prefix.into()); self }
}

#[derive(Debug, Clone, Default)]
pub struct CacheStats { pub hits: u64, pub misses: u64, pub sets: u64, pub deletes: u64, pub errors: u64 }

impl CacheStats {
    pub fn hit_ratio(&self) -> f64 { let total = self.hits + self.misses; if total == 0 { 0.0 } else { self.hits as f64 / total as f64 } }
}

struct AtomicStats { hits: AtomicU64, misses: AtomicU64, sets: AtomicU64, deletes: AtomicU64, errors: AtomicU64 }
impl AtomicStats {
    fn new() -> Self { Self { hits: AtomicU64::new(0), misses: AtomicU64::new(0), sets: AtomicU64::new(0), deletes: AtomicU64::new(0), errors: AtomicU64::new(0) } }
    fn to_stats(&self) -> CacheStats { CacheStats { hits: self.hits.load(Ordering::Relaxed), misses: self.misses.load(Ordering::Relaxed), sets: self.sets.load(Ordering::Relaxed), deletes: self.deletes.load(Ordering::Relaxed), errors: self.errors.load(Ordering::Relaxed) } }
}

pub struct CacheManager { config: CacheConfig, backend: Box<dyn CacheBackend>, stats: Arc<AtomicStats> }

impl CacheManager {
    pub fn new(config: CacheConfig, backend: Box<dyn CacheBackend>) -> Self { Self { config, backend, stats: Arc::new(AtomicStats::new()) } }

    pub async fn get<T: DeserializeOwned>(&self, key: &CacheKey) -> Result<Option<T>> {
        if !self.config.enabled { return Ok(None); }
        let prefixed = self.prefix_key(key);
        match self.backend.get(&prefixed).await {
            Ok(Some(data)) => {
                self.stats.hits.fetch_add(1, Ordering::Relaxed);
                match serde_json::from_slice(&data) {
                    Ok(val) => Ok(Some(val)),
                    Err(_) => { self.stats.errors.fetch_add(1, Ordering::Relaxed); Ok(None) }
                }
            }
            Ok(None) => { self.stats.misses.fetch_add(1, Ordering::Relaxed); Ok(None) }
            Err(e) => { self.stats.errors.fetch_add(1, Ordering::Relaxed); Err(e) }
        }
    }

    pub async fn set<T: Serialize>(&self, key: &CacheKey, value: &T) -> Result<()> { self.set_with_ttl(key, value, self.config.default_ttl).await }

    pub async fn set_with_ttl<T: Serialize>(&self, key: &CacheKey, value: &T, ttl: Duration) -> Result<()> {
        if !self.config.enabled { return Ok(()); }
        let data = serde_json::to_vec(value)?;
        if data.len() > self.config.max_entry_size { return Ok(()); }
        let prefixed = self.prefix_key(key);
        match self.backend.set(&prefixed, &data, ttl).await { Ok(()) => { self.stats.sets.fetch_add(1, Ordering::Relaxed); Ok(()) } Err(e) => { self.stats.errors.fetch_add(1, Ordering::Relaxed); Err(e) } }
    }

    pub async fn delete(&self, key: &CacheKey) -> Result<bool> {
        if !self.config.enabled { return Ok(false); }
        let prefixed = self.prefix_key(key);
        match self.backend.delete(&prefixed).await { Ok(d) => { if d { self.stats.deletes.fetch_add(1, Ordering::Relaxed); } Ok(d) } Err(e) => { self.stats.errors.fetch_add(1, Ordering::Relaxed); Err(e) } }
    }

    pub fn stats(&self) -> CacheStats { self.stats.to_stats() }
    pub fn backend_name(&self) -> &'static str { self.backend.name() }

    fn prefix_key(&self, key: &CacheKey) -> CacheKey {
        if let Some(ref p) = self.config.key_prefix { CacheKey::new(format!("{}:{}", p, key.hash)) } else { key.clone() }
    }
}
