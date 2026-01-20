//! Core AI client implementation

use crate::client::types::CallStats;
use crate::protocol::ProtocolLoader;
use crate::protocol::ProtocolManifest;
use crate::{Error, ErrorContext, Result};
use std::sync::Arc;

use crate::pipeline::Pipeline;
use crate::transport::HttpTransport;

// Import submodules
use crate::client::validation;

/// Unified AI client that works with any provider through protocol configuration.
pub struct AiClient {
    pub manifest: ProtocolManifest,
    pub transport: Arc<HttpTransport>,
    pub pipeline: Arc<Pipeline>,
    pub loader: Arc<ProtocolLoader>,
    pub(crate) fallbacks: Vec<String>,
    pub(crate) model_id: String,
    pub(crate) strict_streaming: bool,
    pub(crate) feedback: Arc<dyn crate::telemetry::FeedbackSink>,
    pub(crate) inflight: Option<Arc<tokio::sync::Semaphore>>,
    pub(crate) max_inflight: Option<usize>,
    pub(crate) attempt_timeout: Option<std::time::Duration>,
    pub(crate) breaker: Option<Arc<crate::resilience::circuit_breaker::CircuitBreaker>>,
    pub(crate) rate_limiter: Option<Arc<crate::resilience::rate_limiter::RateLimiter>>,
}

/// Unified response format.
#[derive(Debug, Default)]
pub struct UnifiedResponse {
    pub content: String,
    pub tool_calls: Vec<crate::types::tool::ToolCall>,
    pub usage: Option<serde_json::Value>,
}

impl AiClient {
    /// Snapshot current runtime signals (facts only) for application-layer orchestration.
    pub async fn signals(&self) -> crate::client::signals::SignalsSnapshot {
        let inflight = self.inflight.as_ref().and_then(|sem| {
            let max = self.max_inflight?;
            let available = sem.available_permits();
            let in_use = max.saturating_sub(available);
            Some(crate::client::signals::InflightSnapshot {
                max,
                available,
                in_use,
            })
        });

        let rate_limiter = match &self.rate_limiter {
            Some(rl) => Some(rl.snapshot().await),
            None => None,
        };

        let circuit_breaker = match &self.breaker {
            Some(cb) => Some(cb.snapshot()),
            None => None,
        };

        crate::client::signals::SignalsSnapshot {
            inflight,
            rate_limiter,
            circuit_breaker,
        }
    }

    /// Create a new client for a specific model.
    ///
    /// The model identifier should be in the format "provider/model-name"
    /// (e.g., "anthropic/claude-3-5-sonnet")
    pub async fn new(model: &str) -> Result<Self> {
        crate::client::builder::AiClientBuilder::new()
            .build(model)
            .await
    }

    /// Create a new client instance for another model, reusing loader + shared runtime knobs
    /// (feedback, inflight, breaker, rate limiter) for consistent behavior.
    pub(crate) async fn with_model(&self, model: &str) -> Result<Self> {
        // model is in form "provider/model-id"
        let parts: Vec<&str> = model.split('/').collect();
        let model_id = parts
            .get(1)
            .map(|s| s.to_string())
            .unwrap_or_else(|| model.to_string());

        let manifest = self.loader.load_model(model).await?;
        validation::validate_manifest(&manifest, self.strict_streaming)?;

        let transport = Arc::new(crate::transport::HttpTransport::new(&manifest, &model_id)?);
        let pipeline = Arc::new(crate::pipeline::Pipeline::from_manifest(&manifest)?);

        Ok(AiClient {
            manifest,
            transport,
            pipeline,
            loader: self.loader.clone(),
            fallbacks: Vec::new(),
            model_id,
            strict_streaming: self.strict_streaming,
            feedback: self.feedback.clone(),
            inflight: self.inflight.clone(),
            max_inflight: self.max_inflight,
            attempt_timeout: self.attempt_timeout,
            breaker: self.breaker.clone(),
            rate_limiter: self.rate_limiter.clone(),
        })
    }

    /// Create a chat request builder.
    pub fn chat(&self) -> crate::client::chat::ChatRequestBuilder<'_> {
        crate::client::chat::ChatRequestBuilder::new(self)
    }

    /// Execute multiple chat requests concurrently with an optional concurrency limit.
    ///
    /// Notes:
    /// - Results preserve input order.
    /// - Internally uses the same "streaming → UnifiedResponse" path for consistency.
    pub async fn chat_batch(
        &self,
        requests: Vec<crate::client::chat::ChatBatchRequest>,
        concurrency_limit: Option<usize>,
    ) -> Vec<Result<UnifiedResponse>> {
        use futures::StreamExt;

        let n = requests.len();
        if n == 0 {
            return Vec::new();
        }

        let limit = concurrency_limit.unwrap_or(10).max(1);
        let mut out: Vec<Option<Result<UnifiedResponse>>> = (0..n).map(|_| None).collect();

        let results: Vec<(usize, Result<UnifiedResponse>)> =
            futures::stream::iter(requests.into_iter().enumerate())
                .map(|(idx, req)| async move {
                    let mut b = self.chat().messages(req.messages).stream();
                    if let Some(t) = req.temperature {
                        b = b.temperature(t);
                    }
                    if let Some(m) = req.max_tokens {
                        b = b.max_tokens(m);
                    }
                    if let Some(tools) = req.tools {
                        b = b.tools(tools);
                    }
                    if let Some(tc) = req.tool_choice {
                        b = b.tool_choice(tc);
                    }
                    let r = b.execute().await;
                    (idx, r)
                })
                .buffer_unordered(limit)
                .collect()
                .await;

        for (idx, r) in results {
            out[idx] = Some(r);
        }

        out.into_iter()
            .map(|o| {
                o.unwrap_or_else(|| {
                    Err(Error::runtime_with_context(
                        "batch result missing",
                        ErrorContext::new().with_source("batch_executor"),
                    ))
                })
            })
            .collect()
    }

    /// Smart batch execution with a conservative, developer-friendly default heuristic.
    ///
    /// - For very small batches, run sequentially to reduce overhead.
    /// - For larger batches, run with a bounded concurrency.
    ///
    /// You can override the chosen concurrency via env:
    /// - `AI_LIB_BATCH_CONCURRENCY`
    pub async fn chat_batch_smart(
        &self,
        requests: Vec<crate::client::chat::ChatBatchRequest>,
    ) -> Vec<Result<UnifiedResponse>> {
        let n = requests.len();
        if n == 0 {
            return Vec::new();
        }

        let env_override = std::env::var("AI_LIB_BATCH_CONCURRENCY")
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .filter(|v| *v > 0);

        let chosen = env_override.unwrap_or_else(|| {
            if n <= 3 {
                1
            } else if n <= 10 {
                5
            } else {
                10
            }
        });

        self.chat_batch(requests, Some(chosen)).await
    }

    /// Report user feedback (optional). This delegates to the injected `FeedbackSink`.
    pub async fn report_feedback(&self, event: crate::telemetry::FeedbackEvent) -> Result<()> {
        self.feedback.report(event).await
    }

    /// Update rate limiter state from response headers using protocol-mapped names.
    ///
    /// This method is public for testing purposes.
    pub async fn update_rate_limits(&self, headers: &reqwest::header::HeaderMap) {
        use crate::client::preflight::PreflightExt;
        PreflightExt::update_rate_limits(self, headers).await;
    }

    /// Unified entry point for calling a model.
    /// Handles text, streaming, and error fallback automatically.
    pub async fn call_model(
        &self,
        request: crate::protocol::UnifiedRequest,
    ) -> Result<UnifiedResponse> {
        Ok(self.call_model_with_stats(request).await?.0)
    }

    /// Call a model and also return per-call stats (latency, retries, request ids, endpoint, usage, etc.).
    ///
    /// This is intended for higher-level model selection and observability.
    /// Call a model and also return per-call stats (latency, retries, request ids, endpoint, usage, etc.).
    ///
    /// This is intended for higher-level model selection and observability.
    pub async fn call_model_with_stats(
        &self,
        request: crate::protocol::UnifiedRequest,
    ) -> Result<(UnifiedResponse, CallStats)> {
        // v0.5.0: The resilience logic is now delegated to the "Resilience Layer" (Pipeline Operators).
        // This core loop is now significantly simpler: it just tries the primary client.
        // If advanced resilience (multi-candidate fallback, complex retries) is needed,
        // it should be configured via the `Pipeline` or `PolicyEngine` which now acts as an operator.

        // Note: For v0.5.0 migration, we preserve the basic fallback iteration here
        // until the `Pipeline` fully absorbs "Client Switching" logic.
        // However, the explicit *retry* loop inside each candidate is now conceptually
        // part of `execute_once_with_stats` (which will eventually use RetryOperator).

        let mut last_err: Option<Error> = None;

        // Build fallback clients first (async)
        // In v0.6.0+, this will be replaced by `FallbackOperator` inside the pipeline
        let mut fallback_clients: Vec<AiClient> = Vec::with_capacity(self.fallbacks.len());
        for model in &self.fallbacks {
            if let Ok(c) = self.with_model(model).await {
                fallback_clients.push(c);
            }
        }

        // Iterate candidates: primary first, then fallbacks.
        for (candidate_idx, client) in std::iter::once(self)
            .chain(fallback_clients.iter())
            .enumerate()
        {
            let has_fallback = candidate_idx + 1 < (1 + fallback_clients.len());
            let policy = crate::client::policy::PolicyEngine::new(&client.manifest);

            // 1. Validation check
            if let Err(e) = policy.validate_capabilities(&request) {
                if has_fallback {
                    last_err = Some(e);
                    continue; // Fallback to next candidate
                } else {
                    return Err(e); // No more fallbacks, fail fast
                }
            }

            // 2. Pre-decision based on signals
            let sig = client.signals().await;
            if let Some(crate::client::policy::Decision::Fallback) =
                policy.pre_decide(&sig, has_fallback)
            {
                last_err = Some(Error::runtime_with_context(
                    "skipped candidate due to signals",
                    ErrorContext::new().with_source("policy_engine"),
                ));
                continue;
            }

            let mut req = request.clone();
            req.model = client.model_id.clone();

            // 3. Execution with Retry Policy
            // The `execute_with_retry` helper now encapsulates the retry loop,
            // paving the way for `RetryOperator` migration.
            match client.execute_with_retry(&req, &policy, has_fallback).await {
                Ok(res) => return Ok(res),
                Err(e) => {
                    // If we are here, retries were exhausted or policy said Fallback/Fail.
                    last_err = Some(e);
                    // If policy said Fallback, continue loop.
                    // If policy said Fail, strictly we should stop, but current logic implies
                    // the loop itself is the "Fallback mechanism".
                    if !has_fallback {
                        return Err(last_err.unwrap());
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            Error::runtime_with_context(
                "all attempts failed",
                ErrorContext::new().with_source("retry_policy"),
            )
        }))
    }

    /// Internal helper to execute with retry policy.
    /// In future versions, this Logic moves entirely into `RetryOperator`.
    async fn execute_with_retry(
        &self,
        request: &crate::protocol::UnifiedRequest,
        policy: &crate::client::policy::PolicyEngine,
        has_fallback: bool,
    ) -> Result<(UnifiedResponse, CallStats)> {
        let mut attempt: u32 = 0;
        let mut retry_count: u32 = 0;

        loop {
            let attempt_fut = self.execute_once_with_stats(request);
            let attempt_res = if let Some(t) = self.attempt_timeout {
                match tokio::time::timeout(t, attempt_fut).await {
                    Ok(r) => r,
                    Err(_) => Err(Error::runtime_with_context(
                        "attempt timeout",
                        ErrorContext::new().with_source("timeout_policy"),
                    )),
                }
            } else {
                attempt_fut.await
            };

            match attempt_res {
                Ok((resp, mut stats)) => {
                    stats.retry_count = retry_count;
                    return Ok((resp, stats));
                }
                Err(e) => {
                    let decision = policy.decide(&e, attempt, has_fallback)?;

                    match decision {
                        crate::client::policy::Decision::Retry { delay } => {
                            retry_count = retry_count.saturating_add(1);
                            if delay.as_millis() > 0 {
                                tokio::time::sleep(delay).await;
                            }
                            attempt = attempt.saturating_add(1);
                            continue;
                        }
                        crate::client::policy::Decision::Fallback => return Err(e),
                        crate::client::policy::Decision::Fail => return Err(e),
                    }
                }
            }
        }
    }

    /// Validate request capabilities.
    pub fn validate_request(
        &self,
        request: &crate::client::chat::ChatRequestBuilder<'_>,
    ) -> Result<()> {
        // Build a minimal UnifiedRequest to check capabilities via PolicyEngine
        let mut mock_req = crate::protocol::UnifiedRequest::default();
        mock_req.stream = request.stream;
        mock_req.tools = request.tools.clone();
        mock_req.messages = request.messages.clone();

        let policy = crate::client::policy::PolicyEngine::new(&self.manifest);
        policy.validate_capabilities(&mock_req)
    }
}
