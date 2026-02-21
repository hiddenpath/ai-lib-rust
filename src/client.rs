//! 统一客户端接口：提供协议驱动的 AI 模型交互入口。
//!
//! Unified client interface for AI-Protocol runtime.
//!
//! Developer-friendly goal: keep the public surface small and predictable.
//! Implementation details are split into submodules under `src/client/`.

pub mod builder;
pub mod chat;
pub mod core;
pub mod endpoint;
pub mod error_classification;
mod execution;
mod policy;
mod preflight;
pub mod signals;
pub mod types;
mod validation;

pub use builder::AiClientBuilder;
pub use chat::{ChatBatchRequest, ChatRequestBuilder};
pub use core::{AiClient, UnifiedResponse};
pub use endpoint::EndpointExt;
pub use signals::SignalsSnapshot;
pub use types::{CallStats, CancelHandle, ClientMetrics};
pub use error_classification::classify_error_from_response;
