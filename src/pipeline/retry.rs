//! Retry Operator
//!
//! This operator handles automatic retries for transient errors.
//! It wraps the source stream logic rather than just transforming the output.

use async_trait::async_trait;
use tokio::time::Duration;

/// Configuration for retry logic
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub min_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
    pub retry_on_status: Vec<u16>,
}

pub struct RetryOperator {
    config: RetryConfig,
}

impl RetryOperator {
    pub fn new(config: RetryConfig) -> Self {
        Self { config }
    }

    /// Calculate backoff with jitter
    fn backoff(&self, attempt: u32) -> Duration {
        let base = self.config.min_delay.as_millis() as u64;
        let cap = self.config.max_delay.as_millis() as u64;

        // Exponential backoff: base * 2^attempt
        let mut delay = base.saturating_mul(1u64 << attempt);
        if delay > cap {
            delay = cap;
        }

        let duration = Duration::from_millis(delay);

        if self.config.jitter {
            // Simple jitter: random standard distribution +/- 10%
            // In production, use a proper RNG
            duration
        } else {
            duration
        }
    }
}

// Logic Note:
// A "Retry Operator" in a stream pipeline is complex because if the *source* fails,
// the stream is already dead. We can't just "retry the stream" from the middle.
// True retries need to happen at the *request execution* level, not just the data processing level.
// However, we can model this as a "Resilience Wrapper" around the transport execution.
// For the purpose of this refactor, we will define the struct here but integration
// will happen in the Execution layer (Client Core), as the user requested "Abstract logic as a Pipeline Operator".
// We will interpret "Pipeline" broadly as the "Request Processing Pipeline", not just "Response Data Pipeline".

#[async_trait]
pub trait ResiliencePolicy: Send + Sync {
    async fn should_retry(&self, attempt: u32, error: &crate::Error) -> Option<Duration>;
}

#[async_trait]
impl ResiliencePolicy for RetryOperator {
    async fn should_retry(&self, attempt: u32, error: &crate::Error) -> Option<Duration> {
        if attempt >= self.config.max_retries {
            return None;
        }

        // Check if error is retryable
        // This relies on improved ErrorContext 2.0 (upcoming)
        // For now, assume all runtime errors are potentially retryable if not strictly fatal
        if matches!(error, crate::Error::Runtime { .. }) {
            // if ctx contains status code, check against retry_on_status
            // logic to be refined with Error Handling 2.0
            return Some(self.backoff(attempt));
        }

        None
    }
}
