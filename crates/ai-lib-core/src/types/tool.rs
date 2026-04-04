//! Tool calling definitions based on AI-Protocol standard_schema

use serde::{Deserialize, Serialize};

/// Tool definition (for function calling)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String, // "function"
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Option<serde_json::Value>, // JSON Schema
}

/// Tool call (invocation from model)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}

/// Tool result (response to tool call)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_use_id: String,
    pub content: serde_json::Value,
    #[serde(default)]
    pub is_error: bool,
}
