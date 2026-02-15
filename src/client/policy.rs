use crate::{Error, Result};
use std::time::Duration;

use crate::client::signals::SignalsSnapshot;

/// Internal decision for how to proceed after a failed attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Decision {
    Retry { delay: Duration },
    Fallback,
    Fail,
}

/// Internal policy engine that unifies retry / fallback behavior.
///
/// Important constraints:
/// - Keep this internal (no public API commitments yet).
/// - Prefer deterministic, explainable behavior over clever heuristics.
pub(crate) struct PolicyEngine {
    manifest: crate::protocol::ProtocolManifest,
    pub max_retries: u32,
    pub min_delay_ms: u32,
    pub max_delay_ms: u32,
}

impl PolicyEngine {
    pub fn new(manifest: &crate::protocol::ProtocolManifest) -> Self {
        let retry = manifest.retry_policy.as_ref();
        let max_retries = retry.and_then(|p| p.max_retries).unwrap_or(0);
        let min_delay_ms = retry.and_then(|p| p.min_delay_ms).unwrap_or(0);
        let max_delay_ms = retry.and_then(|p| p.max_delay_ms).unwrap_or(min_delay_ms);
        Self {
            manifest: manifest.clone(),
            max_retries,
            min_delay_ms,
            max_delay_ms,
        }
    }

    /// Validates if the manifest supports all capabilities required by the request.
    ///
    /// This is a pre-flight guard that validates user intent against protocol capabilities
    /// before making any network requests, saving latency and cost.
    pub fn validate_capabilities(&self, request: &crate::protocol::UnifiedRequest) -> Result<()> {
        let manifest = &self.manifest;

        // Check for Tooling support
        if !request.tools.as_ref().map(|t: &Vec<crate::types::tool::ToolDefinition>| t.is_empty()).unwrap_or(true) {
            if !manifest.supports_capability("tools") {
                return Err(Error::validation_with_context(
                    "Model does not support tool calling",
                    crate::ErrorContext::new()
                        .with_field_path("request.tools")
                        .with_source("capability_validator"),
                ));
            }
        }

        // Check for Streaming support
        if request.stream && !manifest.supports_capability("streaming") {
            return Err(Error::validation_with_context(
                "Model does not support streaming",
                crate::ErrorContext::new()
                    .with_field_path("request.stream")
                    .with_source("capability_validator"),
            ));
        }

        // Check for Multimodal support (Vision/Audio)
        let has_multimodal = request
            .messages
            .iter()
            .any(|m: &crate::types::message::Message| m.contains_image() || m.contains_audio());
        if has_multimodal {
            let supports_multimodal = manifest.supports_capability("multimodal")
                || manifest.supports_capability("vision")
                || manifest.supports_capability("audio");

            if !supports_multimodal {
                return Err(Error::validation_with_context(
                    "Model does not support multimodal content (images/audio)",
                    crate::ErrorContext::new()
                        .with_field_path("request.messages")
                        .with_source("capability_validator"),
                ));
            }
        }

        // Parameter range validation (pre-flight guard to avoid invalid requests)
        // Note: Currently, parameter constraints are not defined in the protocol manifest.
        // This is a placeholder for future enhancement when capabilities include constraints.
        // For now, we rely on provider APIs to reject invalid parameters.

        Ok(())
    }

    fn backoff_delay(&self, attempt: u32, retry_after_ms: Option<u32>) -> Duration {
        let base = if self.min_delay_ms == 0 {
            0
        } else {
            // exponential backoff: min_delay * 2^attempt
            let factor = 1u32.checked_shl(attempt).unwrap_or(u32::MAX);
            self.min_delay_ms.saturating_mul(factor)
        };
        let chosen = retry_after_ms.unwrap_or(base).min(self.max_delay_ms);
        Duration::from_millis(chosen as u64)
    }

    /// Optional pre-decision based on current runtime signals (facts), before attempting a call.
    ///
    /// Keep this conservative: only skip work that is *known* to fail right now.
    pub fn pre_decide(&self, signals: &SignalsSnapshot, has_fallback: bool) -> Option<Decision> {
        if !has_fallback {
            return None;
        }

        let cb_open = signals
            .circuit_breaker
            .as_ref()
            .and_then(|s| s.open_remaining_ms)
            .is_some();
        if cb_open {
            return Some(Decision::Fallback);
        }

        // If this candidate is currently saturated (no inflight permits),
        // prefer trying a fallback candidate rather than waiting here.
        if let Some(inflight) = signals.inflight.as_ref() {
            if inflight.available == 0 {
                return Some(Decision::Fallback);
            }
        }

        // If rate limiter predicts a meaningful wait, prefer fallback rather than sleeping.
        // Keep this threshold conservative: don't bounce models for tiny waits.
        const RATE_LIMIT_FALLBACK_THRESHOLD_MS: u64 = 1_000;
        if let Some(rl) = signals.rate_limiter.as_ref() {
            if let Some(wait_ms) = rl.estimated_wait_ms {
                if wait_ms >= RATE_LIMIT_FALLBACK_THRESHOLD_MS {
                    return Some(Decision::Fallback);
                }
            }
        }

        None
    }

    /// Decide what to do next after an attempt failed.
    ///
    /// - `attempt` is 0-based (first failure => attempt=0).
    /// - `has_fallback` indicates there is another candidate to try.
    pub fn decide(&self, err: &Error, attempt: u32, has_fallback: bool) -> Result<Decision> {
        let (mut retryable, mut fallbackable, retry_after_ms) = match err {
            Error::Remote {
                retryable,
                fallbackable,
                retry_after_ms,
                ..
            } => (*retryable, *fallbackable, *retry_after_ms),
            Error::Transport(_) => (true, true, None),
            Error::Runtime { message: msg, .. } => {
                // Preflight and guard errors are policy-relevant.
                // Keep these rules simple and explainable:
                // - circuit breaker open => try fallback if available
                // - attempt timeout => retry and/or fallback
                let m = msg.to_lowercase();
                if m.contains("circuit breaker open") {
                    (false, true, None)
                } else if m.contains("timeout") {
                    (true, true, None)
                } else {
                    (false, false, None)
                }
            }
            _ => (false, false, None),
        };

        // Prefer ErrorContext 2.0 flags when present
        if let Some(ctx) = err.context() {
            if let Some(r) = ctx.retryable {
                retryable = r;
            }
            if let Some(f) = ctx.fallbackable {
                fallbackable = f;
            }
        }

        if retryable && attempt < self.max_retries {
            return Ok(Decision::Retry {
                delay: self.backoff_delay(attempt, retry_after_ms),
            });
        }

        if fallbackable && has_fallback {
            return Ok(Decision::Fallback);
        }

        Ok(Decision::Fail)
    }
}
