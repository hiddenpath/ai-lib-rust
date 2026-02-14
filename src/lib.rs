//! # ai-lib-rust
//!
//! 这是 AI-Protocol 规范的高性能 Rust 参考实现，提供统一的多厂商 AI 模型交互接口。
//!
//! Protocol Runtime for AI-Protocol - A high-performance Rust reference implementation
//! that enables provider-agnostic AI model interactions.
//!
//! ## Overview
//!
//! This library implements the AI-Protocol specification as a runtime, where all logic
//! is operators and all configuration is protocol. It provides a unified interface
//! for interacting with AI models across different providers without hardcoding
//! provider-specific logic.
//!
//! ## Core Philosophy
//!
//! - **Protocol-Driven**: All behavior is configured through protocol manifests, not code
//! - **Provider-Agnostic**: Unified interface across OpenAI, Anthropic, Google, and others
//! - **Streaming-First**: Native support for Server-Sent Events (SSE) streaming
//! - **Type-Safe**: Strongly typed request/response handling with comprehensive error types
//!
//! ## Key Features
//!
//! - **Unified Client**: [`AiClient`] provides a single entry point for all AI interactions
//! - **Protocol Loading**: Load and validate protocol manifests from local files or remote URLs
//! - **Streaming Pipeline**: Configurable operator pipeline for response processing
//! - **Batching**: Efficient request batching with [`batch::BatchCollector`]
//! - **Caching**: Response caching with pluggable backends via [`cache`] module
//! - **Resilience**: Circuit breaker and rate limiting via [`resilience`] module
//! - **Content Safety**: Guardrails for content filtering via [`guardrails`] module
//! - **Telemetry**: Optional feedback collection via [`telemetry`] module
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use ai_lib_rust::{AiClient, AiClientBuilder, Message, MessageRole};
//!
//! #[tokio::main]
//! async fn main() -> ai_lib_rust::Result<()> {
//!     let client = AiClientBuilder::new()
//!         .with_protocol_path("protocols/openai.yaml")?
//!         .with_api_key("your-api-key")
//!         .build()?;
//!
//!     let messages = vec![
//!         Message::user("Hello, how are you?"),
//!     ];
//!
//!     // Streaming response
//!     let mut stream = client.chat_stream(&messages, None).await?;
//!     // Process stream events...
//!     
//!     Ok(())
//! }
//! ```
//!
//! ## Module Organization
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`protocol`] | Protocol specification loading and validation |
//! | [`client`] | AI client implementation and builders |
//! | [`pipeline`] | Streaming response pipeline operators |
//! | [`types`] | Core type definitions (messages, events, tools) |
//! | [`batch`] | Request batching and parallel execution |
//! | [`cache`] | Response caching with multiple backends |
//! | [`embeddings`] | Embedding generation and vector operations |
//! | [`resilience`] | Circuit breaker and rate limiting |
//! | [`guardrails`] | Content filtering and safety checks |
//! | [`tokens`] | Token counting and cost estimation |
//! | [`telemetry`] | Optional feedback and telemetry collection |

pub mod batch;
pub mod cache;
pub mod client;
pub mod embeddings;
pub mod guardrails;
pub mod pipeline;
pub mod plugins;
pub mod protocol;
pub mod resilience;
pub mod structured;
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

/// A unified pinned, boxed stream that emits `PipeResult<T>`
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = PipeResult<T>> + Send + 'a>>;

/// Error type for the library
pub mod error;
pub use error::{Error, ErrorContext};
