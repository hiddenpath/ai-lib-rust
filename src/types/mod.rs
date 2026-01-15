//! Standard type system based on AI-Protocol standard_schema

pub mod events;
pub mod message;
pub mod tool;

pub use events::StreamingEvent;
pub use message::{Message, MessageRole};
pub use tool::{ToolCall, ToolDefinition};
