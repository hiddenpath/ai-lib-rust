//! 响应缓存模块：提供可插拔的缓存后端以减少重复 API 调用。
//!
//! # Response Caching Module
//!
//! This module provides flexible response caching capabilities with pluggable
//! backends, reducing API calls and improving response times for repeated requests.
//!
//! ## Overview
//!
//! Caching is valuable for:
//! - Reducing API costs by avoiding duplicate requests
//! - Improving response latency for repeated queries
//! - Enabling offline or degraded mode operation
//! - Supporting development and testing workflows
//!
//! ## Key Components
//!
//! | Component | Description |
//! |-----------|-------------|
//! | [`CacheManager`] | High-level cache management with TTL and statistics |
//! | [`CacheConfig`] | Configuration for cache behavior and limits |
//! | [`CacheBackend`] | Trait for implementing custom cache backends |
//! | [`MemoryCache`] | In-memory LRU cache implementation |
//! | [`NullCache`] | No-op cache for disabling caching |
//! | [`CacheKey`] | Cache key generation from request parameters |
//!
//! ## Example
//!
//! ```rust
//! use ai_lib_rust::cache::{CacheManager, CacheConfig, MemoryCache};
//! use std::time::Duration;
//!
//! // Create an in-memory cache with 1-hour TTL
//! let backend = MemoryCache::new(1000); // max 1000 entries
//! let config = CacheConfig {
//!     ttl: Duration::from_secs(3600),
//!     enabled: true,
//!     ..Default::default()
//! };
//! let cache = CacheManager::new(Box::new(backend), config);
//! ```
//!
//! ## Cache Key Generation
//!
//! Cache keys are generated from:
//! - Model identifier
//! - Message content (hashed)
//! - Request parameters (temperature, max_tokens, etc.)
//!
//! This ensures identical requests return cached responses while different
//! parameters generate new cache entries.

mod backend;
mod key;
mod manager;

pub use backend::{CacheBackend, MemoryCache, NullCache};
pub use key::{CacheKey, CacheKeyGenerator};
pub use manager::{CacheConfig, CacheManager, CacheStats};
