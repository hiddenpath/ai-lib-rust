//! Cache backend implementations.

use super::key::CacheKey;
use crate::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Clone)]
struct CacheEntry {
    data: Vec<u8>,
    created_at: Instant,
    ttl: Duration,
    last_accessed: Instant,
}

impl CacheEntry {
    fn new(data: Vec<u8>, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            data,
            created_at: now,
            ttl,
            last_accessed: now,
        }
    }
    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }
}

#[async_trait]
pub trait CacheBackend: Send + Sync {
    async fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>>;
    async fn set(&self, key: &CacheKey, value: &[u8], ttl: Duration) -> Result<()>;
    async fn delete(&self, key: &CacheKey) -> Result<bool>;
    async fn exists(&self, key: &CacheKey) -> Result<bool>;
    async fn clear(&self) -> Result<()>;
    async fn len(&self) -> Result<usize>;
    fn name(&self) -> &'static str;
}

pub struct MemoryCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    max_entries: usize,
}

impl MemoryCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            max_entries,
        }
    }
    fn evict_if_needed(&self, entries: &mut HashMap<String, CacheEntry>) {
        entries.retain(|_, e| !e.is_expired());
        while entries.len() >= self.max_entries {
            let oldest = entries
                .iter()
                .min_by_key(|(_, e)| e.last_accessed)
                .map(|(k, _)| k.clone());
            if let Some(k) = oldest {
                entries.remove(&k);
            } else {
                break;
            }
        }
    }
}

#[async_trait]
impl CacheBackend for MemoryCache {
    async fn get(&self, key: &CacheKey) -> Result<Option<Vec<u8>>> {
        let mut entries = self.entries.write().unwrap();
        if let Some(entry) = entries.get_mut(&key.hash) {
            if entry.is_expired() {
                entries.remove(&key.hash);
                return Ok(None);
            }
            entry.last_accessed = Instant::now();
            return Ok(Some(entry.data.clone()));
        }
        Ok(None)
    }
    async fn set(&self, key: &CacheKey, value: &[u8], ttl: Duration) -> Result<()> {
        let mut entries = self.entries.write().unwrap();
        self.evict_if_needed(&mut entries);
        entries.insert(key.hash.clone(), CacheEntry::new(value.to_vec(), ttl));
        Ok(())
    }
    async fn delete(&self, key: &CacheKey) -> Result<bool> {
        Ok(self.entries.write().unwrap().remove(&key.hash).is_some())
    }
    async fn exists(&self, key: &CacheKey) -> Result<bool> {
        let entries = self.entries.read().unwrap();
        Ok(entries
            .get(&key.hash)
            .map(|e| !e.is_expired())
            .unwrap_or(false))
    }
    async fn clear(&self) -> Result<()> {
        self.entries.write().unwrap().clear();
        Ok(())
    }
    async fn len(&self) -> Result<usize> {
        Ok(self
            .entries
            .read()
            .unwrap()
            .values()
            .filter(|e| !e.is_expired())
            .count())
    }
    fn name(&self) -> &'static str {
        "memory"
    }
}

pub struct NullCache;
impl NullCache {
    pub fn new() -> Self {
        Self
    }
}
impl Default for NullCache {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CacheBackend for NullCache {
    async fn get(&self, _: &CacheKey) -> Result<Option<Vec<u8>>> {
        Ok(None)
    }
    async fn set(&self, _: &CacheKey, _: &[u8], _: Duration) -> Result<()> {
        Ok(())
    }
    async fn delete(&self, _: &CacheKey) -> Result<bool> {
        Ok(false)
    }
    async fn exists(&self, _: &CacheKey) -> Result<bool> {
        Ok(false)
    }
    async fn clear(&self) -> Result<()> {
        Ok(())
    }
    async fn len(&self) -> Result<usize> {
        Ok(0)
    }
    fn name(&self) -> &'static str {
        "null"
    }
}
