//! MCP 工具桥接模块 — 将 MCP 服务器工具转换为 AI-Protocol 统一工具格式
//!
//! MCP (Model Context Protocol) tool bridge module. Converts tools exposed by
//! MCP servers into the AI-Protocol unified `ToolDefinition` format, and maps
//! AI-Protocol `ToolCall` / `ToolResult` back to MCP wire format.
//!
//! This module handles:
//! - MCP tool → AI-Protocol `ToolDefinition` conversion
//! - AI-Protocol `ToolCall` → MCP tool invocation format
//! - Provider-specific MCP configuration (headers, tool types, endpoints)
//! - Tool filtering (allow/deny lists) from manifest declarations

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

use crate::protocol::v2::manifest::McpConfig;
use crate::types::tool::{FunctionDefinition, ToolCall, ToolDefinition, ToolResult};

/// An MCP tool as received from an MCP server's `tools/list` response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpTool {
    /// Tool name (MCP spec: must be unique within a server).
    pub name: String,
    /// Human-readable description.
    #[serde(default)]
    pub description: Option<String>,
    /// JSON Schema describing the tool's input parameters.
    #[serde(default, rename = "inputSchema")]
    pub input_schema: Option<Value>,
}

/// An MCP tool invocation request (sent to an MCP server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInvocation {
    pub name: String,
    pub arguments: Value,
}

/// An MCP tool invocation result (received from an MCP server).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolResult {
    /// Result content (may be text, JSON, or structured).
    pub content: Vec<McpContent>,
    /// Whether the tool execution resulted in an error.
    #[serde(default)]
    pub is_error: bool,
}

/// MCP content block within a tool result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpContent {
    #[serde(rename = "type")]
    pub content_type: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

/// MCP server connection descriptor — used by the client manager to connect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerSpec {
    /// Human-readable server name.
    pub name: String,
    /// Transport type (stdio | sse | streamable_http).
    pub transport: String,
    /// Connection URI or command.
    pub uri: String,
    /// Optional authentication.
    #[serde(default)]
    pub auth: Option<McpAuth>,
}

/// MCP server authentication configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpAuth {
    pub method: String,
    #[serde(default)]
    pub token: Option<String>,
    #[serde(default)]
    pub token_env: Option<String>,
}

/// Provider-specific MCP request configuration — used by ProviderDriver.
#[derive(Debug, Clone)]
pub struct McpProviderConfig {
    /// Provider's tool type identifier (e.g., "mcp" for OpenAI).
    pub tool_type: String,
    /// Required beta header, if any (e.g., Anthropic's "mcp-client-2025-11-20").
    pub beta_header: Option<String>,
    /// API endpoint supporting MCP (may differ from chat endpoint).
    pub api_endpoint: Option<String>,
    /// How MCP servers are configured in the provider API.
    pub config_method: McpConfigMethod,
}

/// How MCP servers are configured in a provider's API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpConfigMethod {
    /// MCP config passed as a tool parameter in the request body.
    ToolParameter,
    /// MCP config set via SDK/client initialization.
    SdkConfig,
    /// MCP config set via CLI flag.
    CliFlag,
}

/// Tool bridge: converts between MCP tools and AI-Protocol tools.
///
/// The bridge is stateless and manifest-driven: it reads the provider's MCP
/// configuration to determine naming conventions, filtering rules, and
/// provider-specific formatting.
#[derive(Debug)]
pub struct McpToolBridge {
    /// Namespace prefix for MCP tools (e.g., "mcp__servername__").
    namespace: String,
    /// Allowed tools (empty = all allowed).
    allow_filter: HashSet<String>,
    /// Denied tools.
    deny_filter: HashSet<String>,
}

impl McpToolBridge {
    /// Create a new bridge for a specific MCP server.
    pub fn new(server_name: &str) -> Self {
        Self {
            namespace: format!("mcp__{}__", server_name),
            allow_filter: HashSet::new(),
            deny_filter: HashSet::new(),
        }
    }

    /// Set allowed tools filter.
    pub fn with_allow_filter(mut self, tools: impl IntoIterator<Item = String>) -> Self {
        self.allow_filter = tools.into_iter().collect();
        self
    }

    /// Set denied tools filter.
    pub fn with_deny_filter(mut self, tools: impl IntoIterator<Item = String>) -> Self {
        self.deny_filter = tools.into_iter().collect();
        self
    }

    /// Convert a list of MCP tools to AI-Protocol `ToolDefinition`s.
    ///
    /// Applies allow/deny filtering and namespaces the tool names
    /// to prevent collisions between multiple MCP servers.
    pub fn mcp_tools_to_protocol(&self, mcp_tools: &[McpTool]) -> Vec<ToolDefinition> {
        mcp_tools
            .iter()
            .filter(|t| self.is_tool_allowed(&t.name))
            .map(|t| self.convert_tool(t))
            .collect()
    }

    /// Convert a single MCP tool to AI-Protocol format.
    fn convert_tool(&self, tool: &McpTool) -> ToolDefinition {
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: self.namespaced_name(&tool.name),
                description: tool.description.clone(),
                parameters: tool.input_schema.clone(),
            },
        }
    }

    /// Convert an AI-Protocol `ToolCall` back to MCP invocation format.
    ///
    /// Strips the namespace prefix and extracts the original MCP tool name.
    pub fn protocol_call_to_mcp(&self, call: &ToolCall) -> Option<McpToolInvocation> {
        let original_name = self.strip_namespace(&call.name)?;
        Some(McpToolInvocation {
            name: original_name,
            arguments: call.arguments.clone(),
        })
    }

    /// Convert an MCP tool result to an AI-Protocol `ToolResult`.
    pub fn mcp_result_to_protocol(
        &self,
        tool_call_id: &str,
        result: &McpToolResult,
    ) -> ToolResult {
        let content = result
            .content
            .iter()
            .filter_map(|c| c.text.clone())
            .collect::<Vec<_>>()
            .join("\n");

        ToolResult {
            tool_use_id: tool_call_id.to_string(),
            content: if result.is_error {
                serde_json::json!({ "error": content })
            } else {
                serde_json::json!(content)
            },
            is_error: result.is_error,
        }
    }

    /// Check if a tool name passes the allow/deny filters.
    fn is_tool_allowed(&self, name: &str) -> bool {
        if !self.deny_filter.is_empty() && self.deny_filter.contains(name) {
            return false;
        }
        if !self.allow_filter.is_empty() {
            return self.allow_filter.contains(name);
        }
        true
    }

    /// Add the server namespace prefix to a tool name.
    fn namespaced_name(&self, name: &str) -> String {
        format!("{}{}", self.namespace, name)
    }

    /// Strip the server namespace prefix from a namespaced tool name.
    fn strip_namespace(&self, namespaced: &str) -> Option<String> {
        namespaced
            .strip_prefix(&self.namespace)
            .map(String::from)
    }
}

/// Extract provider-specific MCP configuration from a manifest.
pub fn extract_provider_config(mcp_config: &McpConfig) -> Option<McpProviderConfig> {
    let client = mcp_config.client.as_ref()?;
    if !client.supported {
        return None;
    }
    let mapping = client.provider_mapping.as_ref();

    let tool_type = mapping
        .and_then(|m| m.get("tool_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("mcp")
        .to_string();

    let beta_header = mapping
        .and_then(|m| m.get("beta_header"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let api_endpoint = mapping
        .and_then(|m| m.get("api_endpoint"))
        .and_then(|v| v.as_str())
        .map(String::from);

    let config_method = mapping
        .and_then(|m| m.get("config_method"))
        .and_then(|v| v.as_str())
        .map(|s| match s {
            "sdk_config" => McpConfigMethod::SdkConfig,
            "cli_flag" => McpConfigMethod::CliFlag,
            _ => McpConfigMethod::ToolParameter,
        })
        .unwrap_or(McpConfigMethod::ToolParameter);

    Some(McpProviderConfig {
        tool_type,
        beta_header,
        api_endpoint,
        config_method,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_mcp_tools() -> Vec<McpTool> {
        vec![
            McpTool {
                name: "read_file".into(),
                description: Some("Read a file from disk".into()),
                input_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": { "path": { "type": "string" } },
                    "required": ["path"]
                })),
            },
            McpTool {
                name: "search".into(),
                description: Some("Search the web".into()),
                input_schema: Some(serde_json::json!({
                    "type": "object",
                    "properties": { "query": { "type": "string" } },
                    "required": ["query"]
                })),
            },
            McpTool {
                name: "exec_dangerous".into(),
                description: Some("Execute shell command".into()),
                input_schema: None,
            },
        ]
    }

    #[test]
    fn test_mcp_to_protocol_conversion() {
        let bridge = McpToolBridge::new("fileserver");
        let tools = bridge.mcp_tools_to_protocol(&sample_mcp_tools());
        assert_eq!(tools.len(), 3);
        assert_eq!(tools[0].function.name, "mcp__fileserver__read_file");
        assert_eq!(tools[0].tool_type, "function");
        assert!(tools[0].function.parameters.is_some());
    }

    #[test]
    fn test_tool_filtering_allow() {
        let bridge = McpToolBridge::new("srv")
            .with_allow_filter(vec!["read_file".into(), "search".into()]);
        let tools = bridge.mcp_tools_to_protocol(&sample_mcp_tools());
        assert_eq!(tools.len(), 2);
        assert!(tools.iter().all(|t| !t.function.name.contains("exec_dangerous")));
    }

    #[test]
    fn test_tool_filtering_deny() {
        let bridge = McpToolBridge::new("srv")
            .with_deny_filter(vec!["exec_dangerous".into()]);
        let tools = bridge.mcp_tools_to_protocol(&sample_mcp_tools());
        assert_eq!(tools.len(), 2);
    }

    #[test]
    fn test_protocol_call_to_mcp() {
        let bridge = McpToolBridge::new("srv");
        let call = ToolCall {
            id: "call_123".into(),
            name: "mcp__srv__read_file".into(),
            arguments: serde_json::json!({"path": "/tmp/test.txt"}),
        };
        let invocation = bridge.protocol_call_to_mcp(&call).unwrap();
        assert_eq!(invocation.name, "read_file");
        assert_eq!(invocation.arguments["path"], "/tmp/test.txt");
    }

    #[test]
    fn test_protocol_call_wrong_namespace() {
        let bridge = McpToolBridge::new("srv");
        let call = ToolCall {
            id: "call_1".into(),
            name: "mcp__other__read_file".into(),
            arguments: Value::Null,
        };
        assert!(bridge.protocol_call_to_mcp(&call).is_none());
    }

    #[test]
    fn test_mcp_result_to_protocol() {
        let bridge = McpToolBridge::new("srv");
        let result = McpToolResult {
            content: vec![McpContent {
                content_type: "text".into(),
                text: Some("file contents here".into()),
                extra: HashMap::new(),
            }],
            is_error: false,
        };
        let proto = bridge.mcp_result_to_protocol("call_123", &result);
        assert_eq!(proto.tool_use_id, "call_123");
        assert!(!proto.is_error);
    }

    #[test]
    fn test_mcp_result_error() {
        let bridge = McpToolBridge::new("srv");
        let result = McpToolResult {
            content: vec![McpContent {
                content_type: "text".into(),
                text: Some("file not found".into()),
                extra: HashMap::new(),
            }],
            is_error: true,
        };
        let proto = bridge.mcp_result_to_protocol("call_1", &result);
        assert!(proto.is_error);
        assert!(proto.content["error"].as_str().unwrap().contains("file not found"));
    }

    #[test]
    fn test_extract_provider_config() {
        use crate::protocol::v2::manifest::McpClientConfig;
        let config = McpConfig {
            client: Some(McpClientConfig {
                supported: true,
                protocol_version: Some("2025-11-25".into()),
                transports: vec!["sse".into()],
                auth_methods: vec![],
                capabilities: None,
                tool_filtering: None,
                approval_modes: vec![],
                provider_mapping: Some(HashMap::from([
                    ("tool_type".into(), Value::String("mcp".into())),
                    ("beta_header".into(), Value::String("mcp-client-2025-11-20".into())),
                    ("config_method".into(), Value::String("tool_parameter".into())),
                ])),
            }),
            server: None,
        };
        let prov = extract_provider_config(&config).unwrap();
        assert_eq!(prov.tool_type, "mcp");
        assert_eq!(prov.beta_header.as_deref(), Some("mcp-client-2025-11-20"));
        assert_eq!(prov.config_method, McpConfigMethod::ToolParameter);
    }
}
