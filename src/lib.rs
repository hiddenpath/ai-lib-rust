//! # ai-lib-rust
//!
//! Protocol Runtime for AI-Protocol - A high-performance Rust reference implementation.
//!
//! This library implements the AI-Protocol specification as a runtime, where all logic
//! is operators and all configuration is protocol. It provides a unified interface
//! for interacting with AI models across different providers without hardcoding
//! provider-specific logic.

pub mod batch;
pub mod cache;
pub mod client;
pub mod embeddings;
pub mod pipeline;
pub mod plugins;
pub mod protocol;
pub mod resilience;
pub mod telemetry;
pub mod tokens;
pub mod transport;
pub mod types;
pub mod utils;

#[cfg(feature = "routing_mvp")]
pub mod routing;

#[cfg(feature = "interceptors")]
pub mod interceptors;

// Re-export main types for convenience
pub use client::CallStats;
pub use client::CancelHandle;
pub use client::ChatBatchRequest;
pub use client::EndpointExt;
pub use client::{AiClient, AiClientBuilder};
pub use telemetry::{FeedbackEvent, FeedbackSink};
pub use types::{
    events::StreamingEvent,
    message::{Message, MessageRole},
    tool::ToolCall,
};

// Optional re-exports
#[cfg(feature = "routing_mvp")]
pub use routing::{
    CustomModelManager, LoadBalancingStrategy, ModelArray, ModelCapabilities, ModelEndpoint,
    ModelInfo, ModelSelectionStrategy, PerformanceMetrics, PricingInfo, QualityTier, SpeedTier,
};

use futures::Stream;
use std::pin::Pin;

/// Result type alias for the library
pub type Result<T> = std::result::Result<T, Error>;

/// A specialized Result for pipeline operations
pub type PipeResult<T> = std::result::Result<T, Error>;

/// A unified pinned, boxed stream that emits PipeResult<T>
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = PipeResult<T>> + Send + 'a>>;

/// Error type for the library
pub mod error;
pub use error::{Error, ErrorContext};
