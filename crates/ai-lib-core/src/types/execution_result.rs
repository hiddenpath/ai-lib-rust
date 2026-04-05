//! E/P 边界：执行层返回结果类型（对应 Paper1 第 3 章 3.1–3.2）。
//!
//! E/P boundary — execution result types (Paper1 section 3.1–3.2).
//!
//! The **execution layer (E)** returns [`ExecutionResult`] with [`ExecutionMetadata`].
//! The **contact / policy layer (P)** consumes metadata for routing, retry, and
//! degradation; E does not interpret policy outcomes.
//!
//! ## Micro-retry (E-only, bounded)
//!
//! E MAY perform at most **1–2** automatic retries for **transient transport**
//! failures (e.g. connection reset, HTTP 429/503 where the manifest marks retryable,
//! read timeout) before surfacing an error to P. E MUST NOT implement
//! cross-provider fallback, circuit-breaker-driven retry loops, or policy-selected
//! retry budgets — those belong in P.

use serde::{Deserialize, Serialize};

use crate::error_code::StandardErrorCode;

/// Token usage attached to an execution (aligned with driver [`UsageInfo`](crate::drivers::UsageInfo)).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionUsage {
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub total_tokens: u64,
    /// OpenAI-style `completion_tokens_details.reasoning_tokens`.
    pub reasoning_tokens: Option<u64>,
    /// Anthropic cache read / creation input tokens when present.
    pub cache_read_tokens: Option<u64>,
    pub cache_creation_tokens: Option<u64>,
}

/// Metadata returned with every E-layer call for P-layer policy decisions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionMetadata {
    pub provider_id: String,
    pub model_id: String,
    pub execution_latency_ms: u64,
    pub translation_latency_ms: u64,
    /// Count of **micro-retries** (bounded, transport-level) attempted inside E.
    pub micro_retry_count: u8,
    pub error_code: Option<StandardErrorCode>,
    pub usage: Option<ExecutionUsage>,
}

impl ExecutionMetadata {
    /// Creates minimal metadata with zero latencies and no usage.
    pub fn minimal(provider_id: impl Into<String>, model_id: impl Into<String>) -> Self {
        Self {
            provider_id: provider_id.into(),
            model_id: model_id.into(),
            execution_latency_ms: 0,
            translation_latency_ms: 0,
            micro_retry_count: 0,
            error_code: None,
            usage: None,
        }
    }
}

/// Successful execution envelope from E: payload plus metadata for P.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExecutionResult<T> {
    pub data: T,
    pub metadata: ExecutionMetadata,
}

impl<T> ExecutionResult<T> {
    /// Maps the payload while preserving metadata.
    pub fn map<U, F: FnOnce(T) -> U>(self, f: F) -> ExecutionResult<U> {
        ExecutionResult {
            data: f(self.data),
            metadata: self.metadata,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_result_roundtrip_json() {
        let meta = ExecutionMetadata {
            provider_id: "mock-openai".into(),
            model_id: "gpt-test".into(),
            execution_latency_ms: 10,
            translation_latency_ms: 2,
            micro_retry_count: 1,
            error_code: Some(StandardErrorCode::RateLimited),
            usage: Some(ExecutionUsage {
                prompt_tokens: 3,
                completion_tokens: 5,
                total_tokens: 8,
                reasoning_tokens: None,
                cache_read_tokens: None,
                cache_creation_tokens: None,
            }),
        };
        let er = ExecutionResult {
            data: "hello".to_string(),
            metadata: meta,
        };
        let json = serde_json::to_string(&er).unwrap();
        let back: ExecutionResult<String> = serde_json::from_str(&json).unwrap();
        assert_eq!(back.data, "hello");
        assert_eq!(
            back.metadata.error_code,
            Some(StandardErrorCode::RateLimited)
        );
    }
}
