//! # ai-lib-rust
//!
//! 这是 AI-Protocol 规范的高性能 Rust 参考实现（工作区聚合 crate：`ai-lib-core` 执行层 + `ai-lib-contact` 策略层）。
//!
//! Protocol Runtime for AI-Protocol — workspace facade over `ai-lib-core` and `ai-lib-contact`.

pub use ai_lib_core::*;

pub use ai_lib_contact::cache;
pub use ai_lib_contact::plugins;
pub use ai_lib_contact::resilience;

#[cfg(feature = "batch")]
pub use ai_lib_contact::batch;
#[cfg(feature = "guardrails")]
pub use ai_lib_contact::guardrails;
#[cfg(feature = "tokens")]
pub use ai_lib_contact::tokens;
#[cfg(feature = "telemetry")]
pub use ai_lib_contact::telemetry;
#[cfg(feature = "routing_mvp")]
pub use ai_lib_contact::routing;
#[cfg(feature = "interceptors")]
pub use ai_lib_contact::interceptors;

#[cfg(feature = "routing_mvp")]
pub use ai_lib_contact::routing::{
    CustomModelManager, LoadBalancingStrategy, ModelArray, ModelCapabilities, ModelEndpoint,
    ModelInfo, ModelSelectionStrategy, PerformanceMetrics, PricingInfo, QualityTier, SpeedTier,
};
