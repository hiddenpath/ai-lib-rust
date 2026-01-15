use crate::client::types::CallStats;
use crate::protocol::{EndpointConfig, ProtocolLoader, ProtocolManifest};
use crate::types::events::StreamingEvent;
use crate::{Error, Result};
use futures::TryStreamExt;
use reqwest::header::HeaderMap;
use std::pin::Pin;
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::pipeline::Pipeline;
use crate::transport::HttpTransport;
use tokio::sync::OwnedSemaphorePermit;

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
    fn header_first(headers: &HeaderMap, names: &[&str]) -> Option<String> {
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
    fn retry_after_ms(headers: &HeaderMap) -> Option<u32> {
        let raw = Self::header_first(headers, &["retry-after"])?;
        let secs: u32 = raw.parse().ok()?;
        Some(secs.saturating_mul(1000))
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
        AiClient::validate_manifest(&manifest, self.strict_streaming)?;

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
    pub fn chat(&self) -> crate::client::chat::ChatRequestBuilder {
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
            .map(|o| o.unwrap_or_else(|| Err(Error::runtime("batch result missing"))))
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

    /// Unified policy preflight for a request:
    /// - rate limiter (optional)
    /// - circuit breaker allow (optional)
    /// - inflight backpressure permit (optional)
    pub(crate) async fn preflight(&self) -> Result<Option<OwnedSemaphorePermit>> {
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
                    .map_err(|_| Error::runtime("Backpressure semaphore closed"))?,
            ));
        }
        Ok(None)
    }

    /// Update rate limiter state from response headers using protocol-mapped names.
    pub async fn update_rate_limits(&self, headers: &HeaderMap) {
        if let Some(rl) = &self.rate_limiter {
            if let Some(conf) = &self.manifest.rate_limit_headers {
                // 1. Try Retry-After (highest priority for 429/overload)
                if let Some(name) = &conf.retry_after {
                    if let Some(v) = Self::header_first(headers, &[name]) {
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
                    .and_then(|h| Self::header_first(headers, &[h]))
                    .and_then(|s| s.parse::<u64>().ok());

                let reset_after = conf
                    .requests_reset
                    .as_ref()
                    .and_then(|h| Self::header_first(headers, &[h]))
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

    pub(crate) fn on_success(&self) {
        if let Some(b) = &self.breaker {
            b.on_success();
        }
    }

    pub(crate) fn on_failure(&self) {
        if let Some(b) = &self.breaker {
            b.on_failure();
        }
    }

    /// Report user feedback (optional). This delegates to the injected `FeedbackSink`.
    pub async fn report_feedback(&self, event: crate::telemetry::FeedbackEvent) -> Result<()> {
        self.feedback.report(event).await
    }

    /// Validate that the manifest supports required capabilities.
    ///
    /// When `strict_streaming` is enabled, this performs fail-fast checks for streaming config
    /// completeness to avoid ambiguous runtime behavior.
    pub(crate) fn validate_manifest(
        manifest: &ProtocolManifest,
        strict_streaming: bool,
    ) -> Result<()> {
        if !strict_streaming {
            return Ok(());
        }

        // If the protocol claims streaming capability, require streaming configuration.
        if manifest.supports_capability("streaming") {
            let streaming = manifest.streaming.as_ref().ok_or_else(|| {
                Error::Validation("strict_streaming: manifest.streaming is required".to_string())
            })?;

            let decoder = streaming.decoder.as_ref().ok_or_else(|| {
                Error::Validation("strict_streaming: streaming.decoder is required".to_string())
            })?;
            if decoder.format.trim().is_empty() {
                return Err(Error::Validation(
                    "strict_streaming: streaming.decoder.format must be non-empty".to_string(),
                ));
            }

            // If no explicit event_map rules are provided, the default PathEventMapper needs paths.
            if streaming.event_map.is_empty() {
                if streaming
                    .content_path
                    .as_deref()
                    .map(|s| s.trim().is_empty())
                    .unwrap_or(true)
                {
                    return Err(Error::Validation(
                        "strict_streaming: streaming.content_path is required when streaming.event_map is empty"
                            .to_string(),
                    ));
                }

                if manifest.supports_capability("tools")
                    && streaming
                        .tool_call_path
                        .as_deref()
                        .map(|s| s.trim().is_empty())
                        .unwrap_or(true)
                {
                    return Err(Error::Validation(
                        "strict_streaming: streaming.tool_call_path is required for tools when streaming.event_map is empty"
                            .to_string(),
                    ));
                }
            }
        }

        Ok(())
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
    pub async fn call_model_with_stats(
        &self,
        request: crate::protocol::UnifiedRequest,
    ) -> Result<(UnifiedResponse, CallStats)> {
        // Unified policy-driven loop:
        // - tries the primary client first, then fallbacks
        // - for each candidate, retries are handled consistently based on that candidate's manifest
        // - request.model is forced to match the candidate model_id
        let mut last_err: Option<Error> = None;

        // Build fallback clients first (async), then run a unified decision loop:
        // primary -> fallbacks (in order).
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
            let mut attempt: u32 = 0;
            let mut retry_count: u32 = 0;

            loop {
                // Decision loop for a single candidate.

                // 1. Validation check: Does this manifest support the capabilities?
                // If it doesn't, skip this candidate and try fallback.
                if let Err(e) = policy.validate_capabilities(&request) {
                    if has_fallback {
                        last_err = Some(e);
                        break; // Fallback to next candidate
                    } else {
                        return Err(e); // No more fallbacks, fail fast
                    }
                }

                // Pre-decision based on signals (skip known-bad candidates, e.g. breaker open).
                let sig = client.signals().await;
                if let Some(crate::client::policy::Decision::Fallback) =
                    policy.pre_decide(&sig, has_fallback)
                {
                    last_err = Some(Error::runtime("skipped candidate due to signals"));
                    break;
                }

                let mut req = request.clone();
                req.model = client.model_id.clone();

                let attempt_fut = client.execute_once_with_stats(&req);
                let attempt_res = if let Some(t) = client.attempt_timeout {
                    match tokio::time::timeout(t, attempt_fut).await {
                        Ok(r) => r,
                        Err(_) => Err(Error::runtime("attempt timeout")),
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
                        last_err = Some(e);

                        match decision {
                            crate::client::policy::Decision::Retry { delay } => {
                                retry_count = retry_count.saturating_add(1);
                                if delay.as_millis() > 0 {
                                    tokio::time::sleep(delay).await;
                                }
                                attempt = attempt.saturating_add(1);
                                continue;
                            }
                            crate::client::policy::Decision::Fallback => break,
                            crate::client::policy::Decision::Fail => {
                                return Err(last_err.unwrap());
                            }
                        }
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| Error::runtime("all attempts failed")))
    }

    /// Start a streaming request and return the event stream.
    ///
    /// This is a single attempt (no retry/fallback). Higher-level policy loops live in the caller.
    pub(crate) async fn execute_stream_once<'a>(
        &self,
        request: &crate::protocol::UnifiedRequest,
    ) -> Result<(
        Pin<Box<dyn futures::stream::Stream<Item = Result<StreamingEvent>> + Send + 'static>>,
        Option<OwnedSemaphorePermit>,
        CallStats,
    )> {
        let permit = self.preflight().await?;
        let client_request_id = Uuid::new_v4().to_string();

        let provider_request = self.manifest.compile_request(request)?;
        let endpoint = self.resolve_endpoint(&request.operation)?;

        let start = std::time::Instant::now();
        let resp = self
            .transport
            .execute_stream_response(
                &endpoint.method,
                &endpoint.path,
                &provider_request,
                Some(&client_request_id),
            )
            .await?;

        // Extract rate limits immediately from any response (success or error)
        self.update_rate_limits(resp.headers()).await;

        if !resp.status().is_success() {
            self.on_failure();
            let status = resp.status().as_u16();
            let class = self
                .manifest
                .error_classification
                .as_ref()
                .and_then(|ec| ec.by_http_status.as_ref())
                .and_then(|m| m.get(&status.to_string()).cloned())
                .unwrap_or_else(|| "http_error".to_string());

            // Protocol-driven fallback decision: use standard error_classes guidance
            // from spec.yaml. Transient errors (retryable) are typically fallbackable.
            let should_fallback = Self::is_fallbackable_error_class(class.as_str());

            let headers = resp.headers().clone();
            let retry_after_ms = Self::retry_after_ms(&headers);
            let body = resp.text().await.unwrap_or_default();

            let retry_policy = self.manifest.retry_policy.as_ref();
            let retryable = retry_policy
                .and_then(|p| p.retry_on_http_status.as_ref())
                .map(|v| v.contains(&status))
                .unwrap_or(false);

            info!(
                http_status = status,
                error_class = class.as_str(),
                endpoint = endpoint.path.as_str(),
                duration_ms = start.elapsed().as_millis(),
                "ai-lib-rust streaming request failed"
            );

            return Err(Error::Remote {
                status,
                class,
                message: body,
                retryable,
                fallbackable: should_fallback,
                retry_after_ms,
            });
        }

        self.on_success();

        let upstream_request_id = Self::header_first(
            resp.headers(),
            &["x-request-id", "request-id", "x-amzn-requestid", "cf-ray"],
        );
        let http_status = resp.status().as_u16();

        let response_stream: crate::BoxStream<'static, bytes::Bytes> = Box::pin(
            resp.bytes_stream()
                .map_err(|e| Error::Transport(crate::transport::TransportError::Http(e))),
        );
        let event_stream = self
            .pipeline
            .clone()
            .process_stream_arc(response_stream)
            .await?;

        let stats = CallStats {
            model: request.model.clone(),
            operation: request.operation.clone(),
            endpoint: endpoint.path.clone(),
            http_status,
            retry_count: 0,
            duration_ms: start.elapsed().as_millis(),
            first_event_ms: None,
            emitted_any: false,
            client_request_id,
            upstream_request_id,
            error_class: None,
            usage: None,
            signals: self.signals().await,
        };

        Ok((event_stream, permit, stats))
    }

    async fn execute_once_with_stats(
        &self,
        request: &crate::protocol::UnifiedRequest,
    ) -> Result<(UnifiedResponse, CallStats)> {
        let _permit = self.preflight().await?;

        let client_request_id = Uuid::new_v4().to_string();

        // Compile unified request to provider-specific format
        let provider_request = self.manifest.compile_request(request)?;

        // Resolve endpoint based on request intent (operation)
        let endpoint = self.resolve_endpoint(&request.operation)?;

        let start = std::time::Instant::now();

        let mut last_upstream_request_id: Option<String> = None;
        let resp = self
            .transport
            .execute_stream_response(
                &endpoint.method,
                &endpoint.path,
                &provider_request,
                Some(&client_request_id),
            )
            .await?;

        // Extract rate limits immediately
        self.update_rate_limits(resp.headers()).await;

        // Status-based error classification (protocol-driven) + fallback decision
        if !resp.status().is_success() {
            self.on_failure();
            let status = resp.status().as_u16();
            let class = self
                .manifest
                .error_classification
                .as_ref()
                .and_then(|ec| ec.by_http_status.as_ref())
                .and_then(|m| m.get(&status.to_string()).cloned())
                .unwrap_or_else(|| "http_error".to_string());

            // Protocol-driven fallback decision: use standard error_classes guidance
            // from spec.yaml. Transient errors (retryable) are typically fallbackable.
            let should_fallback = Self::is_fallbackable_error_class(class.as_str());

            let headers = resp.headers().clone();
            let request_id = Self::header_first(
                &headers,
                &["x-request-id", "request-id", "x-amzn-requestid", "cf-ray"],
            );
            let body = resp.text().await.unwrap_or_default();
            info!(
                http_status = status,
                error_class = class.as_str(),
                request_id = request_id.as_deref().unwrap_or(""),
                endpoint = endpoint.path.as_str(),
                duration_ms = start.elapsed().as_millis(),
                "ai-lib-rust request failed"
            );
            let retry_policy = self.manifest.retry_policy.as_ref();
            let retryable = retry_policy
                .and_then(|p| p.retry_on_http_status.as_ref())
                .map(|v| v.contains(&status))
                .unwrap_or(false);
            let retry_after_ms = Self::retry_after_ms(&headers);

            return Err(Error::Remote {
                status,
                class,
                message: body,
                retryable,
                fallbackable: should_fallback,
                retry_after_ms,
            });
        }

        info!(
            http_status = resp.status().as_u16(),
            client_request_id = client_request_id.as_str(),
            endpoint = endpoint.path.as_str(),
            duration_ms = start.elapsed().as_millis(),
            "ai-lib-rust request started streaming"
        );
        self.on_success();

        if last_upstream_request_id.is_none() {
            last_upstream_request_id = Self::header_first(
                resp.headers(),
                &["x-request-id", "request-id", "x-amzn-requestid", "cf-ray"],
            );
        }

        let http_status = resp.status().as_u16();
        let response_stream: crate::BoxStream<'static, bytes::Bytes> = Box::pin(
            resp.bytes_stream()
                .map_err(|e| Error::Transport(crate::transport::TransportError::Http(e))),
        );

        let mut event_stream = self
            .pipeline
            .clone()
            .process_stream_arc(response_stream)
            .await?;

        let mut response = UnifiedResponse::default();
        let mut tool_asm = crate::utils::tool_call_assembler::ToolCallAssembler::new();
        use futures::StreamExt;

        while let Some(event) = event_stream.next().await {
            match event? {
                StreamingEvent::PartialContentDelta { content, .. } => {
                    response.content.push_str(&content);
                }
                StreamingEvent::ToolCallStarted {
                    tool_call_id,
                    tool_name,
                    ..
                } => {
                    tool_asm.on_started(tool_call_id, tool_name);
                }
                StreamingEvent::PartialToolCall {
                    tool_call_id,
                    arguments,
                    ..
                } => {
                    tool_asm.on_partial(&tool_call_id, &arguments);
                }
                StreamingEvent::Metadata { usage, .. } => {
                    response.usage = usage;
                }
                _ => {}
            }
        }

        response.tool_calls = tool_asm.finalize();

        let stats = CallStats {
            model: request.model.clone(),
            operation: request.operation.clone(),
            endpoint: endpoint.path.clone(),
            http_status,
            retry_count: 0,
            duration_ms: start.elapsed().as_millis(),
            first_event_ms: None,
            emitted_any: true,
            client_request_id,
            upstream_request_id: last_upstream_request_id,
            error_class: None,
            usage: response.usage.clone(),
            signals: self.signals().await,
        };

        Ok((response, stats))
    }

    pub(crate) fn resolve_endpoint(&self, name: &str) -> Result<&EndpointConfig> {
        self.manifest
            .endpoints
            .as_ref()
            .and_then(|eps| eps.get(name))
            .ok_or_else(|| {
                Error::Protocol(crate::protocol::ProtocolError::NotFound(format!(
                    "Endpoint '{}' not defined",
                    name
                )))
            })
    }

    pub fn validate_request(
        &self,
        request: &crate::client::chat::ChatRequestBuilder,
    ) -> Result<()> {
        // Build a minimal UnifiedRequest to check capabilities via PolicyEngine
        let mut mock_req = crate::protocol::UnifiedRequest::default();
        mock_req.stream = request.stream;
        mock_req.tools = request.tools.clone();
        mock_req.messages = request.messages.clone();

        let policy = crate::client::policy::PolicyEngine::new(&self.manifest);
        policy.validate_capabilities(&mock_req)
    }

    /// List models available from the provider.
    pub async fn list_remote_models(&self) -> Result<Vec<String>> {
        let response = self.call_service("list_models").await?;

        let models: Vec<String> = if let Some(data) = response.get("data") {
            data.as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| {
                    m.get("id")
                        .and_then(|id| id.as_str().map(|s| s.to_string()))
                })
                .collect()
        } else if let Some(models) = response.get("models") {
            // Gemini style
            models
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| {
                    m.get("name")
                        .and_then(|n| n.as_str().map(|s| s.to_string()))
                })
                .collect()
        } else {
            vec![]
        };

        Ok(models)
    }

    /// Call a generic service by name.
    pub async fn call_service(&self, service_name: &str) -> Result<serde_json::Value> {
        let service = self
            .manifest
            .services
            .as_ref()
            .and_then(|services| services.get(service_name))
            .ok_or_else(|| {
                crate::Error::Protocol(crate::protocol::ProtocolError::NotFound(format!(
                    "Service '{}' not defined",
                    service_name
                )))
            })?;

        self.transport
            .execute_service(
                &service.path,
                &service.method,
                service.headers.as_ref(),
                service.query_params.as_ref(),
            )
            .await
    }

    /// Determine if an error class is fallbackable based on protocol specification.
    ///
    /// This follows the standard error_classes from spec.yaml:
    /// - Transient errors (retryable) are typically fallbackable
    /// - Quota/authentication errors may be fallbackable if another provider is available
    /// - Invalid requests are NOT fallbackable (they'll fail on any provider)
    fn is_fallbackable_error_class(error_class: &str) -> bool {
        // Based on spec.yaml standard_schema.error_handling.error_classes:
        // Transient errors (default_retryable: true) are typically fallbackable
        match error_class {
            // Transient server errors - fallback makes sense
            "rate_limited" | "overloaded" | "server_error" | "timeout" | "conflict" => true,
            // Quota exhausted - may work on another provider
            "quota_exhausted" => true,
            // Client errors - don't fallback (will fail on any provider)
            "invalid_request" | "authentication" | "permission_denied" | "not_found"
            | "request_too_large" | "cancelled" => false,
            // Unknown/other - conservative: don't fallback
            _ => false,
        }
    }
}
