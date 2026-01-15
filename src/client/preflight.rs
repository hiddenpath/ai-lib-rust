//! Preflight checks and rate limit management

use crate::{Error, ErrorContext, Result};
use reqwest::header::HeaderMap;
use tokio::sync::OwnedSemaphorePermit;

use super::core::AiClient;

// These methods are implemented as extension traits to avoid circular dependencies
// They are re-exported in core.rs
pub trait PreflightExt {
    async fn preflight(&self) -> Result<Option<OwnedSemaphorePermit>>;
    async fn update_rate_limits(&self, headers: &HeaderMap);
    fn on_success(&self);
    fn on_failure(&self);
    fn header_first(&self, headers: &HeaderMap, names: &[&str]) -> Option<String>;
    fn retry_after_ms(&self, headers: &HeaderMap) -> Option<u32>;
}

impl PreflightExt for AiClient {
    /// Unified policy preflight for a request:
    /// - rate limiter (optional)
    /// - circuit breaker allow (optional)
    /// - inflight backpressure permit (optional)
    async fn preflight(&self) -> Result<Option<OwnedSemaphorePermit>> {
        // Keep preflight lightweight but unified. Rate limiting and breaker allow are per-call gates,
        // while the inflight permit is held for the whole call/stream lifetime.
        if let Some(rl) = &self.rate_limiter {
            rl.acquire().await?;
        }
        if let Some(b) = &self.breaker {
            b.allow()?;
        }
        if let Some(sem) = &self.inflight {
            return Ok(Some(
                sem.clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| {
                        Error::runtime_with_context(
                            "Backpressure semaphore closed",
                            ErrorContext::new().with_source("backpressure"),
                        )
                    })?,
            ));
        }
        Ok(None)
    }

    /// Update rate limiter state from response headers using protocol-mapped names.
    async fn update_rate_limits(&self, headers: &HeaderMap) {
        if let Some(rl) = &self.rate_limiter {
            if let Some(conf) = &self.manifest.rate_limit_headers {
                // 1. Try Retry-After (highest priority for 429/overload)
                if let Some(name) = &conf.retry_after {
                    if let Some(v) = self.header_first(headers, &[name]) {
                        if let Ok(secs) = v.parse::<u64>() {
                            rl.update_budget(Some(0), Some(std::time::Duration::from_secs(secs)))
                                .await;
                            return;
                        }
                    }
                }

                // 2. Generic Remaining/Reset for requests
                let remaining = conf
                    .requests_remaining
                    .as_ref()
                    .and_then(|h| self.header_first(headers, &[h]))
                    .and_then(|s| s.parse::<u64>().ok());

                let reset_after = conf
                    .requests_reset
                    .as_ref()
                    .and_then(|h| self.header_first(headers, &[h]))
                    .and_then(|s| {
                        if let Ok(val) = s.parse::<u64>() {
                            if val > 1_000_000_000 {
                                // Likely an epoch timestamp
                                let now = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .ok()?
                                    .as_secs();
                                Some(std::time::Duration::from_secs(val.saturating_sub(now)))
                            } else {
                                // Likely seconds or ms? Standardize on seconds for now.
                                Some(std::time::Duration::from_secs(val))
                            }
                        } else {
                            None
                        }
                    });

                rl.update_budget(remaining, reset_after).await;
            }
        }
    }

    fn on_success(&self) {
        if let Some(b) = &self.breaker {
            b.on_success();
        }
    }

    fn on_failure(&self) {
        if let Some(b) = &self.breaker {
            b.on_failure();
        }
    }

    /// Extract the first matching header value from a list of header names.
    fn header_first(&self, headers: &HeaderMap, names: &[&str]) -> Option<String> {
        for name in names {
            if let Some(v) = headers.get(*name) {
                if let Ok(s) = v.to_str() {
                    let s = s.trim();
                    if !s.is_empty() {
                        return Some(s.to_string());
                    }
                }
            }
        }
        None
    }

    /// Best-effort parsing of `Retry-After` header.
    ///
    /// We intentionally only support the common `Retry-After: <seconds>` form to avoid new deps.
    fn retry_after_ms(&self, headers: &HeaderMap) -> Option<u32> {
        let raw = self.header_first(headers, &["retry-after"])?;
        let secs: u32 = raw.parse().ok()?;
        Some(secs.saturating_mul(1000))
    }
}
