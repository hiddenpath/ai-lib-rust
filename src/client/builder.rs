use crate::client::core::AiClient;
use crate::protocol::ProtocolLoader;
use crate::Result;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Builder for creating clients with custom configuration.
///
/// Keep this surface area small and predictable (developer-friendly).
pub struct AiClientBuilder {
    protocol_path: Option<String>,
    hot_reload: bool,
    fallbacks: Vec<String>,
    strict_streaming: bool,
    feedback: Arc<dyn crate::telemetry::FeedbackSink>,
    max_inflight: Option<usize>,
    breaker: Option<Arc<crate::resilience::circuit_breaker::CircuitBreaker>>,
    rate_limiter: Option<Arc<crate::resilience::rate_limiter::RateLimiter>>,
    /// Override base URL (primarily for testing with mock servers)
    base_url_override: Option<String>,
}

impl AiClientBuilder {
    pub fn new() -> Self {
        Self {
            protocol_path: None,
            hot_reload: false,
            fallbacks: Vec::new(),
            strict_streaming: false,
            feedback: crate::telemetry::noop_sink(),
            max_inflight: None,
            breaker: None,
            rate_limiter: None,
            base_url_override: None,
        }
    }

    /// Set custom protocol directory path.
    pub fn protocol_path(mut self, path: String) -> Self {
        self.protocol_path = Some(path);
        self
    }

    /// Enable hot reload of protocol files.
    pub fn hot_reload(mut self, enable: bool) -> Self {
        self.hot_reload = enable;
        self
    }

    /// Set fallback models.
    pub fn with_fallbacks(mut self, fallbacks: Vec<String>) -> Self {
        self.fallbacks = fallbacks;
        self
    }

    /// Enable strict streaming validation (fail fast when streaming config is incomplete).
    ///
    /// This is intentionally opt-in to preserve compatibility with partial manifests.
    pub fn strict_streaming(mut self, enable: bool) -> Self {
        self.strict_streaming = enable;
        self
    }

    /// Inject a feedback sink. Default is a no-op sink.
    pub fn feedback_sink(mut self, sink: Arc<dyn crate::telemetry::FeedbackSink>) -> Self {
        self.feedback = sink;
        self
    }

    /// Limit maximum number of in-flight requests/streams.
    /// This is a simple backpressure mechanism for production safety.
    pub fn max_inflight(mut self, n: usize) -> Self {
        self.max_inflight = Some(n.max(1));
        self
    }

    /// Enable a minimal circuit breaker.
    ///
    /// Defaults can also be enabled via env:
    /// - `AI_LIB_BREAKER_FAILURE_THRESHOLD` (default 5)
    /// - `AI_LIB_BREAKER_COOLDOWN_SECS` (default 30)
    pub fn circuit_breaker_default(mut self) -> Self {
        let threshold = std::env::var("AI_LIB_BREAKER_FAILURE_THRESHOLD")
            .ok()
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(5);
        let cooldown_secs = std::env::var("AI_LIB_BREAKER_COOLDOWN_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(30);
        let cfg = crate::resilience::circuit_breaker::CircuitBreakerConfig {
            failure_threshold: threshold.max(1),
            cooldown: std::time::Duration::from_secs(cooldown_secs.max(1)),
        };
        self.breaker = Some(Arc::new(
            crate::resilience::circuit_breaker::CircuitBreaker::new(cfg),
        ));
        self
    }

    /// Enable a minimal token-bucket rate limiter.
    ///
    /// - Prefer configuring via env to keep API surface small:
    ///   - `AI_LIB_RPS` (requests per second)
    ///   - `AI_LIB_RPM` (requests per minute)
    pub fn rate_limit_rps(mut self, rps: f64) -> Self {
        if let Some(cfg) = crate::resilience::rate_limiter::RateLimiterConfig::from_rps(rps) {
            self.rate_limiter = Some(Arc::new(crate::resilience::rate_limiter::RateLimiter::new(
                cfg,
            )));
        }
        self
    }

    /// Override the base URL from the protocol manifest.
    ///
    /// This is primarily for testing with mock servers. In production, use the
    /// base_url defined in the protocol manifest.
    pub fn base_url_override(mut self, base_url: impl Into<String>) -> Self {
        self.base_url_override = Some(base_url.into());
        self
    }

    /// Build the client.
    pub async fn build(self, model: &str) -> Result<AiClient> {
        let mut loader = ProtocolLoader::new();

        if let Some(path) = self.protocol_path {
            loader = loader.with_base_path(path);
        }

        if self.hot_reload {
            loader = loader.with_hot_reload(true);
        }

        // model is in form "provider/model-id"
        let parts: Vec<&str> = model.split('/').collect();
        let model_id = parts
            .get(1)
            .map(|s| s.to_string())
            .unwrap_or_else(|| model.to_string());

        let manifest = loader.load_model(model).await?;
        let strict_streaming = self.strict_streaming
            || std::env::var("AI_LIB_STRICT_STREAMING").ok().as_deref() == Some("1");
        crate::client::validation::validate_manifest(&manifest, strict_streaming)?;

        let transport = Arc::new(
            crate::transport::HttpTransport::new_with_base_url(
                &manifest,
                &model_id,
                self.base_url_override.as_deref(),
            )?,
        );
        let pipeline = Arc::new(crate::pipeline::Pipeline::from_manifest(&manifest)?);

        let max_inflight = self.max_inflight.or_else(|| {
            std::env::var("AI_LIB_MAX_INFLIGHT")
                .ok()?
                .parse::<usize>()
                .ok()
        });
        let inflight = max_inflight.map(|n| Arc::new(Semaphore::new(n.max(1))));

        // Optional per-attempt timeout (policy signal). Transport has its own timeout too; this is an extra guard.
        let attempt_timeout = std::env::var("AI_LIB_ATTEMPT_TIMEOUT_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|ms| *ms > 0)
            .map(std::time::Duration::from_millis);

        let env_rps = std::env::var("AI_LIB_RPS")
            .ok()
            .and_then(|s| s.parse::<f64>().ok());
        let env_rpm = std::env::var("AI_LIB_RPM")
            .ok()
            .and_then(|s| s.parse::<f64>().ok());
        let env_rate_limiter = env_rps
            .or_else(|| env_rpm.map(|rpm| rpm / 60.0))
            .and_then(crate::resilience::rate_limiter::RateLimiterConfig::from_rps)
            .map(|cfg| Arc::new(crate::resilience::rate_limiter::RateLimiter::new(cfg)));

        // If no explicit rate limiter and manifest has rate limit headers, enable adaptive mode (rps=0)
        let rate_limiter = self.rate_limiter.or(env_rate_limiter).or_else(|| {
            if manifest.rate_limit_headers.is_some() {
                crate::resilience::rate_limiter::RateLimiterConfig::from_rps(0.0)
                    .map(|cfg| Arc::new(crate::resilience::rate_limiter::RateLimiter::new(cfg)))
            } else {
                None
            }
        });

        Ok(AiClient {
            manifest,
            transport,
            pipeline,
            loader: Arc::new(loader),
            fallbacks: self.fallbacks,
            model_id,
            strict_streaming,
            feedback: self.feedback,
            inflight,
            max_inflight,
            attempt_timeout,
            breaker: self.breaker,
            rate_limiter,
        })
    }
}

impl Default for AiClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
