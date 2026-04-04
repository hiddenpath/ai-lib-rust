//! 预检逻辑：请求发送前的背压检查。
//!
//! Preflight checks (inflight backpressure only; rate limiting / circuit breaking live in `ai-lib-contact`).

use crate::{Error, ErrorContext, Result};
use reqwest::header::HeaderMap;
use tokio::sync::OwnedSemaphorePermit;

use super::core::AiClient;

pub trait PreflightExt {
    async fn preflight(&self) -> Result<Option<OwnedSemaphorePermit>>;
    fn header_first(&self, headers: &HeaderMap, names: &[&str]) -> Option<String>;
    fn retry_after_ms(&self, headers: &HeaderMap) -> Option<u32>;
}

impl PreflightExt for AiClient {
    async fn preflight(&self) -> Result<Option<OwnedSemaphorePermit>> {
        if let Some(sem) = &self.inflight {
            return Ok(Some(sem.clone().acquire_owned().await.map_err(|_| {
                Error::runtime_with_context(
                    "Backpressure semaphore closed",
                    ErrorContext::new().with_source("backpressure"),
                )
            })?));
        }
        Ok(None)
    }

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

    fn retry_after_ms(&self, headers: &HeaderMap) -> Option<u32> {
        let raw = self.header_first(headers, &["retry-after"])?;
        let secs: u32 = raw.parse().ok()?;
        Some(secs.saturating_mul(1000))
    }
}
