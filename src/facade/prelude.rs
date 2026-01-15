//! Minimal prelude for application code.
//!
//! Goal: reduce import noise without hiding important concepts.

pub use crate::client::{AiClient, AiClientBuilder, CancelHandle, CallStats, ChatBatchRequest, ChatRequestBuilder};
pub use crate::facade::chat::{ChatCompletionRequest, ChatFacade};
pub use crate::facade::provider::{client_from_provider, ModelRef, Provider};
pub use crate::types::events::StreamingEvent;
pub use crate::types::message::{ContentBlock, Message, MessageContent, MessageRole};

