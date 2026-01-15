//! Unified client interface for AI-Protocol runtime.
//!
//! Developer-friendly goal: keep the public surface small and predictable.
//! Implementation details are split into submodules under `src/client/`.

pub mod builder;
pub mod chat;
pub mod core;
mod policy;
pub mod signals;
pub mod types;

pub use builder::AiClientBuilder;
pub use chat::{ChatBatchRequest, ChatRequestBuilder};
pub use core::{AiClient, UnifiedResponse};
pub use signals::SignalsSnapshot;
pub use types::{CallStats, CancelHandle};

