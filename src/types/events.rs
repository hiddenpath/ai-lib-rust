//! Streaming events based on AI-Protocol standard_schema

use serde::{Deserialize, Serialize};

/// Unified streaming event enum
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type")]
pub enum StreamingEvent {
    /// Partial content delta (text streaming)
    #[serde(rename = "PartialContentDelta")]
    PartialContentDelta {
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_id: Option<u64>,
    },

    /// Thinking delta (reasoning process)
    #[serde(rename = "ThinkingDelta")]
    ThinkingDelta {
        thinking: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_consideration: Option<String>,
    },

    /// Tool call started
    #[serde(rename = "ToolCallStarted")]
    ToolCallStarted {
        tool_call_id: String,
        tool_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },

    /// Partial tool call (arguments streaming)
    #[serde(rename = "PartialToolCall")]
    PartialToolCall {
        tool_call_id: String,
        arguments: String, // Partial JSON string
        #[serde(skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_complete: Option<bool>,
    },

    /// Tool call ended
    #[serde(rename = "ToolCallEnded")]
    ToolCallEnded {
        tool_call_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        index: Option<u32>,
    },

    /// Metadata (usage, finish reason, etc.)
    #[serde(rename = "Metadata")]
    Metadata {
        #[serde(skip_serializing_if = "Option::is_none")]
        usage: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        finish_reason: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        stop_reason: Option<String>,
    },

    /// Final candidate (for multi-candidate scenarios)
    #[serde(rename = "FinalCandidate")]
    FinalCandidate {
        candidate_index: u32,
        finish_reason: String,
    },

    /// Stream end
    #[serde(rename = "StreamEnd")]
    StreamEnd {
        #[serde(skip_serializing_if = "Option::is_none")]
        finish_reason: Option<String>,
    },

    /// Stream error
    #[serde(rename = "StreamError")]
    StreamError {
        error: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        event_id: Option<String>,
    },
}
