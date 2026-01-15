//! Optional interceptor hooks for application-layer cross-cutting concerns.
//!
//! Runtime-first rationale:
//! - The runtime already provides protocol-driven retry/fallback/rate-limiting.
//! - Interceptors are still useful for logging, metrics, auditing, and custom business hooks.
//! - This module intentionally avoids depending on any provider SDK types.

use async_trait::async_trait;

use crate::client::UnifiedResponse;
use crate::protocol::UnifiedRequest;
use crate::Error;

/// Minimal request context passed to interceptors (keep stable to avoid API churn).
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub provider: String,
    pub model: String,
    pub operation: String,
}

/// Response context passed to interceptors.
#[derive(Debug, Clone)]
pub struct ResponseContext {
    pub success: bool,
}

/// Interceptor trait for cross-cutting concerns (logging/metrics/audit/custom behavior).
#[async_trait]
pub trait Interceptor: Send + Sync {
    async fn on_request(&self, _ctx: &RequestContext, _req: &UnifiedRequest) {}

    async fn on_response(
        &self,
        _ctx: &RequestContext,
        _req: &UnifiedRequest,
        _resp: &UnifiedResponse,
    ) {
    }

    async fn on_error(&self, _ctx: &RequestContext, _req: &UnifiedRequest, _err: &Error) {}
}

/// A simple interceptor pipeline that runs hooks in order.
pub struct InterceptorPipeline {
    pub(crate) interceptors: Vec<Box<dyn Interceptor>>,
}

impl InterceptorPipeline {
    pub fn new() -> Self {
        Self {
            interceptors: Vec::new(),
        }
    }

    pub fn with<I: Interceptor + 'static>(mut self, interceptor: I) -> Self {
        self.interceptors.push(Box::new(interceptor));
        self
    }

    /// Run hooks around a provided async function that performs the actual call.
    pub async fn execute<F, Fut>(
        &self,
        ctx: &RequestContext,
        req: &UnifiedRequest,
        f: F,
    ) -> Result<UnifiedResponse, Error>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<UnifiedResponse, Error>>,
    {
        for ic in &self.interceptors {
            ic.on_request(ctx, req).await;
        }

        match f().await {
            Ok(resp) => {
                for ic in &self.interceptors {
                    ic.on_response(ctx, req, &resp).await;
                }
                Ok(resp)
            }
            Err(err) => {
                for ic in &self.interceptors {
                    ic.on_error(ctx, req, &err).await;
                }
                Err(err)
            }
        }
    }
}

impl Default for InterceptorPipeline {
    fn default() -> Self {
        Self::new()
    }
}

