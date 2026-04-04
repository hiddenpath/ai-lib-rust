use crate::client::core::AiClient;
use crate::feedback::FeedbackSink;
use crate::protocol::ProtocolLoader;
use crate::Result;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use tokio::sync::Semaphore;

/// Builder for creating clients with custom configuration.
///
/// Keep this surface area small and predictable (developer-friendly).
///
/// ## Sharing across tasks
///
/// `AiClient` does not implement `Clone` (by design, for API key and ToS compliance).
/// To share a client across multiple async tasks, wrap it in `Arc`:
///
/// ```ignore
/// let client = Arc::new(
///     AiClientBuilder::new()
///         .build("openai/gpt-4o")
///         .await?
/// );
/// // Use Arc::clone(&client) to pass to tasks
/// tokio::spawn(use_client(Arc::clone(&client)));
/// ```
pub struct AiClientBuilder {
    protocol_path: Option<String>,
    hot_reload: bool,
    fallbacks: Vec<String>,
    strict_streaming: bool,
    feedback: Arc<dyn FeedbackSink>,
    max_inflight: Option<usize>,
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
            feedback: crate::feedback::noop_sink(),
            max_inflight: None,
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
    pub fn feedback_sink(mut self, sink: Arc<dyn FeedbackSink>) -> Self {
        self.feedback = sink;
        self
    }

    /// Limit maximum number of in-flight requests/streams.
    /// This is a simple backpressure mechanism for production safety.
    pub fn max_inflight(mut self, n: usize) -> Self {
        self.max_inflight = Some(n.max(1));
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

        // model is in form "provider/model-id" or "provider/org/model-name" (e.g. nvidia/minimaxai/minimax-m2)
        let parts: Vec<&str> = model.split('/').collect();
        let model_id = if parts.len() >= 2 {
            parts[1..].join("/")
        } else {
            model.to_string()
        };

        let manifest = loader.load_model(model).await?;
        let strict_streaming = self.strict_streaming
            || std::env::var("AI_LIB_STRICT_STREAMING").ok().as_deref() == Some("1");
        crate::client::validation::validate_manifest(&manifest, strict_streaming)?;

        // Use MOCK_HTTP_URL env var when base_url_override not set (for testing with ai-protocol-mock)
        let base_url_override = self
            .base_url_override
            .or_else(|| std::env::var("MOCK_HTTP_URL").ok());

        let transport = Arc::new(crate::transport::HttpTransport::new_with_base_url(
            &manifest,
            &model_id,
            base_url_override.as_deref(),
        )?);
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
            total_requests: AtomicU64::new(0),
            successful_requests: AtomicU64::new(0),
            total_tokens: AtomicU64::new(0),
        })
    }
}

impl Default for AiClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
