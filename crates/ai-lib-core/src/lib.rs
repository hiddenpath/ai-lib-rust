//! # ai-lib-core
//!
//! AI-Protocol 执行层：协议加载、客户端、流水线、传输与核心类型。策略模块见 `ai-lib-contact` / 聚合 crate `ai-lib-rust`。
//!
//! Execution-layer runtime for AI-Protocol (protocol, client, pipeline, transport, types).
//!
//! On `wasm32` targets, only protocol parsing, drivers, types, and structured helpers are built
//! (no async client, transport, or pipeline). See PT-072 / `ai-lib-wasm`.

#[cfg(not(target_arch = "wasm32"))]
pub mod client;
pub mod drivers;
#[cfg(not(target_arch = "wasm32"))]
pub mod feedback;
#[cfg(not(target_arch = "wasm32"))]
pub mod pipeline;
pub mod protocol;
#[cfg(not(target_arch = "wasm32"))]
pub mod registry;
pub mod structured;
#[cfg(not(target_arch = "wasm32"))]
pub mod transport;
pub mod types;
pub mod utils;

#[cfg(all(not(target_arch = "wasm32"), feature = "computer_use"))]
pub mod computer_use;
#[cfg(all(not(target_arch = "wasm32"), feature = "embeddings"))]
pub mod embeddings;
#[cfg(all(not(target_arch = "wasm32"), feature = "mcp"))]
pub mod mcp;
#[cfg(all(not(target_arch = "wasm32"), feature = "multimodal"))]
pub mod multimodal;
#[cfg(all(not(target_arch = "wasm32"), feature = "reranking"))]
pub mod rerank;
#[cfg(all(not(target_arch = "wasm32"), feature = "stt"))]
pub mod stt;
#[cfg(all(not(target_arch = "wasm32"), feature = "tts"))]
pub mod tts;

#[cfg(not(target_arch = "wasm32"))]
pub use client::CallStats;
#[cfg(not(target_arch = "wasm32"))]
pub use client::CancelHandle;
#[cfg(not(target_arch = "wasm32"))]
pub use client::ChatBatchRequest;
#[cfg(not(target_arch = "wasm32"))]
pub use client::ClientMetrics;
#[cfg(not(target_arch = "wasm32"))]
pub use client::EndpointExt;
#[cfg(not(target_arch = "wasm32"))]
pub use client::{AiClient, AiClientBuilder};

#[cfg(not(target_arch = "wasm32"))]
pub use feedback::{FeedbackEvent, FeedbackSink};
pub use types::{
    events::StreamingEvent,
    execution_result::{ExecutionMetadata, ExecutionResult, ExecutionUsage},
    message::{Message, MessageRole},
    tool::ToolCall,
};

#[cfg(not(target_arch = "wasm32"))]
use futures::Stream;
#[cfg(not(target_arch = "wasm32"))]
use std::pin::Pin;

/// Result type alias for the library
pub type Result<T> = std::result::Result<T, Error>;

/// A specialized Result for pipeline operations
pub type PipeResult<T> = std::result::Result<T, Error>;

/// A unified pinned, boxed stream that emits `PipeResult<T>`
#[cfg(not(target_arch = "wasm32"))]
pub type BoxStream<'a, T> = Pin<Box<dyn Stream<Item = PipeResult<T>> + Send + 'a>>;

pub mod error;
pub mod error_code;
pub use error::{Error, ErrorContext};
pub use error_code::StandardErrorCode;
