//! Anthropic Messages API 驱动 — 实现 Anthropic 特有的请求/响应格式转换
//!
//! Anthropic Messages API driver. Handles the key differences from OpenAI:
//! - System messages are a top-level `system` parameter, not part of `messages`.
//! - Content uses typed blocks: `[{"type": "text", "text": "..."}]`.
//! - Streaming uses `event: content_block_delta` with `delta.text`.
//! - Response uses `content[0].text` instead of `choices[0].message.content`.
//! - `max_tokens` is required, not optional.

use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

use crate::error::Error;
use crate::protocol::v2::capabilities::Capability;
use crate::protocol::v2::manifest::ApiStyle;
use crate::protocol::ProtocolError;
use crate::types::events::StreamingEvent;
use crate::types::message::{Message, MessageContent, MessageRole};

use super::{DriverRequest, DriverResponse, ProviderDriver, UsageInfo};

const DEFAULT_MAX_TOKENS: u32 = 4096;

/// Anthropic Messages API driver.
#[derive(Debug)]
pub struct AnthropicDriver {
    provider_id: String,
    capabilities: Vec<Capability>,
}

impl AnthropicDriver {
    pub fn new(provider_id: impl Into<String>, capabilities: Vec<Capability>) -> Self {
        Self {
            provider_id: provider_id.into(),
            capabilities,
        }
    }

    /// Extract system message and non-system messages separately.
    /// Anthropic requires system as a top-level param, not in messages array.
    fn split_system_messages(messages: &[Message]) -> (Option<String>, Vec<Value>) {
        let mut system_parts: Vec<String> = Vec::new();
        let mut user_messages: Vec<Value> = Vec::new();

        for m in messages {
            match m.role {
                MessageRole::System => {
                    if let MessageContent::Text(ref s) = m.content {
                        system_parts.push(s.clone());
                    }
                }
                _ => {
                    let role = match m.role {
                        MessageRole::User => "user",
                        MessageRole::Assistant => "assistant",
                        MessageRole::System => unreachable!(),
                    };
                    let content = match &m.content {
                        MessageContent::Text(s) => {
                            serde_json::json!([{ "type": "text", "text": s }])
                        }
                        MessageContent::Blocks(_) => {
                            serde_json::to_value(&m.content).unwrap_or(Value::Null)
                        }
                    };
                    user_messages.push(serde_json::json!({
                        "role": role,
                        "content": content,
                    }));
                }
            }
        }

        let system = if system_parts.is_empty() {
            None
        } else {
            Some(system_parts.join("\n\n"))
        };

        (system, user_messages)
    }
}

#[async_trait]
impl ProviderDriver for AnthropicDriver {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn api_style(&self) -> ApiStyle {
        ApiStyle::AnthropicMessages
    }

    fn build_request(
        &self,
        messages: &[Message],
        model: &str,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
        stream: bool,
        extra: Option<&Value>,
    ) -> Result<DriverRequest, Error> {
        let (system, msgs) = Self::split_system_messages(messages);

        let mut body = serde_json::json!({
            "model": model,
            "messages": msgs,
            "max_tokens": max_tokens.unwrap_or(DEFAULT_MAX_TOKENS),
            "stream": stream,
        });

        if let Some(sys) = system {
            body["system"] = Value::String(sys);
        }
        if let Some(t) = temperature {
            body["temperature"] = serde_json::json!(t);
        }
        if let Some(ext) = extra {
            if let Value::Object(map) = ext {
                for (k, v) in map {
                    body[k] = v.clone();
                }
            }
        }

        let mut headers = HashMap::new();
        headers.insert("anthropic-version".into(), "2023-06-01".into());

        Ok(DriverRequest {
            url: String::new(),
            method: "POST".into(),
            headers,
            body,
            stream,
        })
    }

    fn parse_response(&self, body: &Value) -> Result<DriverResponse, Error> {
        // Anthropic response: { content: [{type: "text", text: "..."}], stop_reason, usage }
        let content = body
            .pointer("/content/0/text")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Normalize stop_reason → finish_reason
        let finish_reason = body
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(|r| match r {
                "end_turn" => "stop".to_string(),
                "max_tokens" => "length".to_string(),
                "tool_use" => "tool_calls".to_string(),
                other => other.to_string(),
            });

        let usage = body.get("usage").map(|u| UsageInfo {
            prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0),
            completion_tokens: u["output_tokens"].as_u64().unwrap_or(0),
            total_tokens: u["input_tokens"].as_u64().unwrap_or(0)
                + u["output_tokens"].as_u64().unwrap_or(0),
        });

        // Extract tool_use blocks from content array
        let tool_calls: Vec<Value> = body
            .get("content")
            .and_then(|c| c.as_array())
            .map(|arr| {
                arr.iter()
                    .filter(|b| b.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default();

        Ok(DriverResponse {
            content,
            finish_reason,
            usage,
            tool_calls,
            raw: body.clone(),
        })
    }

    fn parse_stream_event(&self, data: &str) -> Result<Option<StreamingEvent>, Error> {
        if data.trim().is_empty() {
            return Ok(None);
        }

        let v: Value = serde_json::from_str(data).map_err(|e| {
            Error::Protocol(ProtocolError::ValidationError(format!(
                "Failed to parse Anthropic SSE: {}",
                e
            )))
        })?;

        let event_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match event_type {
            "content_block_delta" => {
                if let Some(text) = v.pointer("/delta/text").and_then(|t| t.as_str()) {
                    if !text.is_empty() {
                        return Ok(Some(StreamingEvent::PartialContentDelta {
                            content: text.to_string(),
                            sequence_id: v.get("index").and_then(|i| i.as_u64()),
                        }));
                    }
                }
                // Thinking delta support
                if let Some(thinking) = v.pointer("/delta/thinking").and_then(|t| t.as_str()) {
                    return Ok(Some(StreamingEvent::ThinkingDelta {
                        thinking: thinking.to_string(),
                        tool_consideration: None,
                    }));
                }
                Ok(None)
            }
            "message_delta" => {
                let reason = v.pointer("/delta/stop_reason").and_then(|r| r.as_str());
                if let Some(r) = reason {
                    return Ok(Some(StreamingEvent::StreamEnd {
                        finish_reason: Some(match r {
                            "end_turn" => "stop".to_string(),
                            "max_tokens" => "length".to_string(),
                            other => other.to_string(),
                        }),
                    }));
                }
                Ok(None)
            }
            "message_stop" => Ok(Some(StreamingEvent::StreamEnd {
                finish_reason: Some("stop".into()),
            })),
            "error" => {
                let error = v.get("error").cloned().unwrap_or(Value::Null);
                Ok(Some(StreamingEvent::StreamError {
                    error,
                    event_id: None,
                }))
            }
            _ => Ok(None),
        }
    }

    fn supported_capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn is_stream_done(&self, _data: &str) -> bool {
        // Anthropic signals done via event type, not a sentinel string.
        // The `event: message_stop` is handled in parse_stream_event.
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_message_extraction() {
        let msgs = vec![
            Message::system("You are helpful."),
            Message::user("Hi"),
        ];
        let (sys, user_msgs) = AnthropicDriver::split_system_messages(&msgs);
        assert_eq!(sys.as_deref(), Some("You are helpful."));
        assert_eq!(user_msgs.len(), 1);
        assert_eq!(user_msgs[0]["role"], "user");
    }

    #[test]
    fn test_anthropic_build_request() {
        let driver = AnthropicDriver::new("anthropic", vec![Capability::Text]);
        let messages = vec![Message::user("Hello")];
        let req = driver
            .build_request(&messages, "claude-sonnet-4-20250514", None, Some(1024), false, None)
            .unwrap();
        assert_eq!(req.body["max_tokens"], 1024);
        assert_eq!(req.body["model"], "claude-sonnet-4-20250514");
        assert!(req.headers.contains_key("anthropic-version"));
    }

    #[test]
    fn test_anthropic_parse_response() {
        let driver = AnthropicDriver::new("anthropic", vec![]);
        let body = serde_json::json!({
            "content": [{"type": "text", "text": "Hello!"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        });
        let resp = driver.parse_response(&body).unwrap();
        assert_eq!(resp.content.as_deref(), Some("Hello!"));
        assert_eq!(resp.finish_reason.as_deref(), Some("stop"));
        assert_eq!(resp.usage.unwrap().total_tokens, 15);
    }

    #[test]
    fn test_anthropic_parse_stream_delta() {
        let driver = AnthropicDriver::new("anthropic", vec![]);
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hi"}}"#;
        let event = driver.parse_stream_event(data).unwrap();
        match event {
            Some(StreamingEvent::PartialContentDelta { content, .. }) => {
                assert_eq!(content, "Hi");
            }
            _ => panic!("Expected PartialContentDelta"),
        }
    }

    #[test]
    fn test_anthropic_stop_reason_normalization() {
        let driver = AnthropicDriver::new("anthropic", vec![]);
        let body = serde_json::json!({
            "content": [{"type": "text", "text": ""}],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 0, "output_tokens": 0}
        });
        let resp = driver.parse_response(&body).unwrap();
        assert_eq!(resp.finish_reason.as_deref(), Some("tool_calls"));
    }
}
