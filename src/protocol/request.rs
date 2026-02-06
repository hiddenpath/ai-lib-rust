//! Unified request format for protocol compilation

/// Unified request format (for protocol compilation)
#[derive(Debug, Clone, Default)]
pub struct UnifiedRequest {
    /// Operation intent used for endpoint routing (e.g. "chat", "completions", "embeddings")
    pub operation: String,
    /// Provider model id (e.g. "deepseek-chat", "gpt-4o-mini")
    pub model: String,
    pub messages: Vec<crate::types::message::Message>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub tools: Option<Vec<crate::types::tool::ToolDefinition>>,
    /// OpenAI-style tool choice. Examples:
    /// - "auto"
    /// - "none"
    /// - {"type":"function","function":{"name":"web_search"}}
    pub tool_choice: Option<serde_json::Value>,
}
