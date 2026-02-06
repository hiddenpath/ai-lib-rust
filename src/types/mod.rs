//! 类型系统模块：定义基于 AI-Protocol 规范的核心数据类型。
//!
//! # Types Module
//!
//! This module defines the core type system based on the AI-Protocol standard schema,
//! providing strongly-typed representations for all AI interaction primitives.
//!
//! ## Overview
//!
//! The type system ensures:
//! - Type-safe message construction and handling
//! - Consistent event representation across providers
//! - Standardized tool/function calling interfaces
//! - Serialization compatibility with AI-Protocol specification
//!
//! ## Key Types
//!
//! | Type | Description |
//! |------|-------------|
//! | [`Message`] | Chat message with role and content |
//! | [`MessageRole`] | Message role (user, assistant, system, tool) |
//! | [`StreamingEvent`] | Unified streaming event representation |
//! | [`ToolCall`] | Function/tool call from model response |
//! | [`ToolDefinition`] | Tool definition for model context |
//!
//! ## Submodules
//!
//! | Module | Description |
//! |--------|-------------|
//! | [`events`] | Streaming event types and variants |
//! | [`message`] | Message types with multi-modal content support |
//! | [`tool`] | Tool/function calling types |
//!
//! ## Example
//!
//! ```rust
//! use ai_lib_rust::types::{Message, MessageRole, ToolDefinition};
//!
//! // Create messages
//! let system = Message::system("You are a helpful assistant");
//! let user = Message::user("What's the weather?");
//!
//! // Define a tool
//! let tool = ToolDefinition {
//!     name: "get_weather".to_string(),
//!     description: Some("Get current weather for a location".to_string()),
//!     parameters: serde_json::json!({
//!         "type": "object",
//!         "properties": {
//!             "location": {"type": "string"}
//!         }
//!     }),
//!     strict: None,
//! };
//! ```

pub mod events;
pub mod message;
pub mod tool;

pub use events::StreamingEvent;
pub use message::{Message, MessageRole};
pub use tool::{ToolCall, ToolDefinition};
