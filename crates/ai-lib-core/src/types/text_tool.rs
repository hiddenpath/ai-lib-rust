//! Text-based tool call parsing for LLMs without reliable native function calling.
//!
//! 文本工具调用解析：适用于不支持或不稳定 native function calling 的 provider。

use super::tool::{ToolCall, ToolDefinition, ToolResult};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

/// Prompt strategy level (L1 standard / L2 counterexamples / L3 few-shot).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum PromptLevel {
    #[default]
    L1,
    L2,
    L3,
}

impl PromptLevel {
    fn parse(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "L2" => Self::L2,
            "L3" => Self::L3,
            _ => Self::L1,
        }
    }
}

/// Configuration for text tool call parsing and prompt generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToolConfig {
    /// Enable lenient parsing (L2-L4 dialect/alias handling).
    #[serde(default)]
    pub lenient_parsing: bool,
    /// Max nesting depth for tool_call blocks.
    #[serde(default = "default_max_depth")]
    pub max_call_depth: u8,
    /// Include counterexample warnings in prompts (L2+).
    #[serde(default = "default_true")]
    pub include_counterexamples: bool,
    /// Prompt strategy level.
    #[serde(default)]
    pub prompt_level: PromptLevel,
    /// Prompt locale: "en" or "zh".
    #[serde(default = "default_locale")]
    pub locale: String,
    /// Preferred JSON key for arguments when normalizing (from manifest args_key).
    #[serde(default)]
    pub args_key: Option<String>,
}

fn default_max_depth() -> u8 {
    1
}
fn default_true() -> bool {
    true
}
fn default_locale() -> String {
    "en".to_string()
}

impl Default for TextToolConfig {
    fn default() -> Self {
        Self {
            lenient_parsing: false,
            max_call_depth: 1,
            include_counterexamples: true,
            prompt_level: PromptLevel::L1,
            locale: "en".to_string(),
            args_key: None,
        }
    }
}

/// Cross-LLM text tool call parser trait.
pub trait TextToolParser: Send + Sync {
    /// Split LLM response into plain text and structured tool calls.
    fn parse(&self, response_text: &str) -> (String, Vec<ToolCall>);

    /// Generate system prompt instructions for tool use protocol.
    fn prompt_instructions(&self, tools: &[ToolDefinition]) -> String;

    /// Format tool execution results for the next LLM turn.
    fn format_results(&self, results: &[ToolResult]) -> String;
}

/// Default implementation using the AI-Protocol standard `<tool_call>` format.
#[derive(Debug, Clone)]
pub struct StandardTextToolParser {
    config: TextToolConfig,
}

impl StandardTextToolParser {
    pub fn new(config: TextToolConfig) -> Self {
        Self { config }
    }

    /// Build parser config from a provider manifest `tool_calling.text_fallback` block.
    pub fn from_manifest_tool_calling(tool_calling: &serde_json::Value) -> Self {
        let mut config = TextToolConfig {
            lenient_parsing: true,
            prompt_level: PromptLevel::L2,
            ..Default::default()
        };

        if let Some(fallback) = tool_calling.get("text_fallback") {
            if let Some(level) = fallback.get("prompt_level").and_then(|v| v.as_str()) {
                config.prompt_level = PromptLevel::parse(level);
            }
            if let Some(key) = fallback.get("args_key").and_then(|v| v.as_str()) {
                config.args_key = Some(key.to_string());
            }
            config.include_counterexamples = config.prompt_level != PromptLevel::L1;
        }

        if let Some(native) = tool_calling.get("native") {
            if native.get("reliability").and_then(|v| v.as_str()) == Some("full") {
                config.lenient_parsing = false;
            }
        }

        Self::new(config)
    }
}

impl TextToolParser for StandardTextToolParser {
    fn parse(&self, response_text: &str) -> (String, Vec<ToolCall>) {
        parse_text_tool_calls(response_text, &self.config)
    }

    fn prompt_instructions(&self, tools: &[ToolDefinition]) -> String {
        generate_prompt_instructions(tools, &self.config)
    }

    fn format_results(&self, results: &[ToolResult]) -> String {
        results
            .iter()
            .map(|r| {
                let body = serde_json::json!({
                    "tool_use_id": r.tool_use_id,
                    "content": r.content,
                    "is_error": r.is_error,
                });
                format!("<tool_result>\n{}\n</tool_result>", body)
            })
            .collect::<Vec<_>>()
            .join("\n")
    }
}

fn tool_call_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)<tool_call(?:\s+[^>]*)?>(.*?)</tool_call>").expect("valid tool_call regex")
    })
}

fn shell_dialect_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"(?s)<shell>\s*<command>(.*?)</command>\s*</shell>")
            .expect("valid shell dialect regex")
    })
}

fn bash_dialect_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"(?s)<bash>(.*?)</bash>").expect("valid bash dialect regex"))
}

fn unwrap_tool_calls_wrapper(text: &str) -> String {
    let outer_re = Regex::new(r"(?s)<tool_calls>\s*(.*?)\s*</tool_calls>").unwrap();
    if let Some(caps) = outer_re.captures(text) {
        caps.get(1)
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| text.to_string())
    } else {
        text.to_string()
    }
}

fn extract_name_from_open_tag(full_match: &str) -> Option<String> {
    let attr_re = Regex::new(r#"name="([^"]+)""#).unwrap();
    attr_re
        .captures(full_match)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn normalize_arguments(obj: &serde_json::Map<String, serde_json::Value>) -> serde_json::Value {
    if obj.contains_key("arguments") {
        return obj
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));
    }
    for key in ["parameters", "params", "args"] {
        if let Some(v) = obj.get(key) {
            return v.clone();
        }
    }
    // Body is the arguments object itself (no wrapper keys).
    let mut args = obj.clone();
    args.remove("name");
    args.remove("id");
    args.remove("type");
    serde_json::Value::Object(args)
}

fn parse_json_body(body: &str, attr_name: Option<String>) -> Option<(String, serde_json::Value)> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(trimmed).ok()?;
    let obj = value.as_object()?;

    let name = obj
        .get("name")
        .and_then(|v| v.as_str())
        .map(String::from)
        .or(attr_name)?;

    let arguments = normalize_arguments(obj);
    Some((name, arguments))
}

fn parse_text_tool_calls(text: &str, config: &TextToolConfig) -> (String, Vec<ToolCall>) {
    let mut tool_calls = Vec::new();
    let mut remaining = text.to_string();

    // L3: unwrap <tool_calls> wrapper when lenient
    if config.lenient_parsing {
        remaining = unwrap_tool_calls_wrapper(&remaining);
    }

    // Collect standard <tool_call> blocks
    let block_re = tool_call_block_re();
    let mut spans_to_remove: Vec<(usize, usize)> = Vec::new();

    for caps in block_re.captures_iter(&remaining) {
        let full = caps.get(0).unwrap();
        let body = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let attr_name = if config.lenient_parsing {
            extract_name_from_open_tag(full.as_str())
        } else {
            None
        };

        if let Some((name, arguments)) = parse_json_body(body, attr_name) {
            let idx = tool_calls.len();
            tool_calls.push(ToolCall {
                id: format!("text_tool_{idx}"),
                name,
                arguments,
            });
            spans_to_remove.push((full.start(), full.end()));
        }
    }

    // L4: dialect adaptation when lenient and no standard blocks found
    if config.lenient_parsing && tool_calls.is_empty() {
        if let Some(caps) = shell_dialect_re().captures(&remaining) {
            let cmd = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            tool_calls.push(ToolCall {
                id: "text_tool_0".to_string(),
                name: "shell".to_string(),
                arguments: serde_json::json!({ "command": cmd }),
            });
            if let Some(full) = caps.get(0) {
                spans_to_remove.push((full.start(), full.end()));
            }
        } else if let Some(caps) = bash_dialect_re().captures(&remaining) {
            let cmd = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            tool_calls.push(ToolCall {
                id: "text_tool_0".to_string(),
                name: "shell".to_string(),
                arguments: serde_json::json!({ "command": cmd }),
            });
            if let Some(full) = caps.get(0) {
                spans_to_remove.push((full.start(), full.end()));
            }
        }
    }

    // Remove matched spans from remaining text (reverse order to preserve indices)
    spans_to_remove.sort_by_key(|(s, _)| *s);
    spans_to_remove.reverse();
    for (start, end) in spans_to_remove {
        if start <= remaining.len() && end <= remaining.len() {
            remaining.replace_range(start..end, "");
        }
    }

    let remaining_text = remaining
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    (remaining_text, tool_calls)
}

fn generate_prompt_instructions(tools: &[ToolDefinition], config: &TextToolConfig) -> String {
    let tool_list = tools
        .iter()
        .map(|t| {
            format!(
                "- {}: {}",
                t.function.name,
                t.function.description.as_deref().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let is_zh = config.locale.starts_with("zh");

    match (config.prompt_level, is_zh) {
        (PromptLevel::L1, true) => format!(
            "## 工具调用协议\n\n\
             <tool_call>\n{{\"name\": \"工具名\", \"arguments\": {{\"参数\": \"值\"}}}}\n</tool_call>\n\n\
             可用工具：\n{tool_list}"
        ),
        (PromptLevel::L1, false) => format!(
            "## Tool Use Protocol\n\n\
             <tool_call>\n{{\"name\": \"tool_name\", \"arguments\": {{\"param\": \"value\"}}}}\n</tool_call>\n\n\
             Available tools:\n{tool_list}"
        ),
        (PromptLevel::L2, true) => format!(
            "## 工具调用协议\n\n\
             <tool_call>\n{{\"name\": \"工具名\", \"arguments\": {{\"参数\": \"值\"}}}}\n</tool_call>\n\n\
             关键规则：\n\
             - 只能使用 <tool_call>。<shell>、<bash>、<function> 将被忽略。\n\
             - JSON 必须包含 \"name\" 和 \"arguments\"。\n\n\
             可用工具：\n{tool_list}"
        ),
        (PromptLevel::L2, false) => format!(
            "## Tool Use Protocol\n\n\
             <tool_call>\n{{\"name\": \"tool_name\", \"arguments\": {{\"param\": \"value\"}}}}\n</tool_call>\n\n\
             CRITICAL RULES:\n\
             - Use <tool_call> ONLY. <shell>, <bash>, <function> WILL BE IGNORED.\n\
             - JSON must contain \"name\" (string) and \"arguments\" (object).\n\
             - Do NOT wrap in <tool_calls> or any other tag.\n\n\
             Available tools:\n{tool_list}"
        ),
        (PromptLevel::L3, _) => format!(
            "## Tool Use Protocol — Example\n\n\
             <tool_call>\n{{\"name\": \"shell\", \"arguments\": {{\"command\": \"ls -la\"}}}}\n</tool_call>\n\n\
             CRITICAL: <shell>, <bash>, <function> formats WILL BE IGNORED.\n\n\
             Available tools:\n{tool_list}"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::tool::FunctionDefinition;

    fn parser_strict() -> StandardTextToolParser {
        StandardTextToolParser::new(TextToolConfig {
            lenient_parsing: false,
            ..Default::default()
        })
    }

    fn parser_lenient() -> StandardTextToolParser {
        StandardTextToolParser::new(TextToolConfig {
            lenient_parsing: true,
            ..Default::default()
        })
    }

    #[test]
    fn strict_parse_standard_format() {
        let text = "I'll list the files for you.\n<tool_call>\n{\"name\": \"shell\", \"arguments\": {\"command\": \"ls -la\"}}\n</tool_call>";
        let (remaining, calls) = parser_strict().parse(text);
        assert_eq!(remaining, "I'll list the files for you.");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "shell");
        assert_eq!(calls[0].arguments["command"], "ls -la");
    }

    #[test]
    fn lenient_attribute_name() {
        let text = r#"<tool_call name="shell">{"command": "ls"}</tool_call>"#;
        let (_, calls) = parser_lenient().parse(text);
        assert_eq!(calls[0].name, "shell");
        assert_eq!(calls[0].arguments["command"], "ls");
    }

    #[test]
    fn lenient_nested_wrapper() {
        let text = r#"<tool_calls><tool_call id="1">{"name": "shell", "parameters": {"command": "ls"}}</tool_call></tool_calls>"#;
        let (_, calls) = parser_lenient().parse(text);
        assert_eq!(calls[0].name, "shell");
        assert_eq!(calls[0].arguments["command"], "ls");
    }

    #[test]
    fn lenient_field_alias() {
        let text = r#"<tool_call>{"name": "search", "params": {"query": "AI protocol", "limit": 10}}</tool_call>"#;
        let (_, calls) = parser_lenient().parse(text);
        assert_eq!(calls[0].name, "search");
        assert_eq!(calls[0].arguments["query"], "AI protocol");
    }

    #[test]
    fn lenient_shell_dialect() {
        let text = "Running command:\n<shell><command>ls</command></shell>";
        let (remaining, calls) = parser_lenient().parse(text);
        assert_eq!(remaining, "Running command:");
        assert_eq!(calls[0].name, "shell");
        assert_eq!(calls[0].arguments["command"], "ls");
    }

    #[test]
    fn prompt_l2_contains_counterexamples() {
        let tools = vec![ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: "shell".to_string(),
                description: Some("Execute shell commands".to_string()),
                parameters: None,
            },
        }];
        let parser = StandardTextToolParser::new(TextToolConfig {
            prompt_level: PromptLevel::L2,
            locale: "en".to_string(),
            ..Default::default()
        });
        let prompt = parser.prompt_instructions(&tools);
        assert!(prompt.contains("<tool_call>"));
        assert!(prompt.contains("WILL BE IGNORED"));
        assert!(prompt.contains("shell"));
    }
}
