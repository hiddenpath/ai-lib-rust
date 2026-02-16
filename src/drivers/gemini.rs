//! Gemini Generate API 驱动 — 实现 Google Gemini 特有的请求/响应格式转换
//!
//! Google Gemini generateContent API driver. Key differences:
//! - Uses `contents` instead of `messages`, with `parts` instead of `content`.
//! - Roles: `user` and `model` (not `assistant`). System uses `system_instruction`.
//! - `generationConfig` wraps temperature, max_tokens (→ `maxOutputTokens`), etc.
//! - Response: `candidates[0].content.parts[0].text`.
//! - Streaming uses NDJSON with the same structure (each line is a full response).
//! - API key is passed as `?key=` query parameter, not in headers.

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

/// Google Gemini generateContent API driver.
#[derive(Debug)]
pub struct GeminiDriver {
    provider_id: String,
    capabilities: Vec<Capability>,
}

impl GeminiDriver {
    pub fn new(provider_id: impl Into<String>, capabilities: Vec<Capability>) -> Self {
        Self {
            provider_id: provider_id.into(),
            capabilities,
        }
    }

    /// Separate system instructions from conversation contents.
    /// Gemini uses `system_instruction` as a top-level field.
    fn split_messages(messages: &[Message]) -> (Option<Value>, Vec<Value>) {
        let mut system_parts: Vec<String> = Vec::new();
        let mut contents: Vec<Value> = Vec::new();

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
                        MessageRole::Assistant => "model",
                        MessageRole::System => unreachable!(),
                    };
                    let parts = Self::content_to_parts(&m.content);
                    contents.push(serde_json::json!({
                        "role": role,
                        "parts": parts,
                    }));
                }
            }
        }

        let system_instruction = if system_parts.is_empty() {
            None
        } else {
            Some(serde_json::json!({
                "parts": [{ "text": system_parts.join("\n\n") }]
            }))
        };

        (system_instruction, contents)
    }

    /// Convert MessageContent to Gemini `parts` array.
    fn content_to_parts(content: &MessageContent) -> Value {
        match content {
            MessageContent::Text(s) => {
                serde_json::json!([{ "text": s }])
            }
            MessageContent::Blocks(_) => {
                // For multimodal blocks, delegate to serde (needs further
                // transformation for Gemini's inline_data format in Sprint 3).
                serde_json::to_value(content).unwrap_or(Value::Null)
            }
        }
    }
}

#[async_trait]
impl ProviderDriver for GeminiDriver {
    fn provider_id(&self) -> &str {
        &self.provider_id
    }

    fn api_style(&self) -> ApiStyle {
        ApiStyle::GeminiGenerate
    }

    fn build_request(
        &self,
        messages: &[Message],
        _model: &str,
        temperature: Option<f64>,
        max_tokens: Option<u32>,
        _stream: bool,
        extra: Option<&Value>,
    ) -> Result<DriverRequest, Error> {
        let (system_instruction, contents) = Self::split_messages(messages);

        let mut body = serde_json::json!({
            "contents": contents,
        });

        if let Some(sys) = system_instruction {
            body["system_instruction"] = sys;
        }

        // Gemini uses `generationConfig` for parameters
        let mut gen_config = serde_json::json!({});
        if let Some(t) = temperature {
            gen_config["temperature"] = serde_json::json!(t);
        }
        if let Some(mt) = max_tokens {
            gen_config["maxOutputTokens"] = serde_json::json!(mt);
        }
        if gen_config != serde_json::json!({}) {
            body["generationConfig"] = gen_config;
        }

        if let Some(ext) = extra {
            if let Value::Object(map) = ext {
                for (k, v) in map {
                    body[k] = v.clone();
                }
            }
        }

        Ok(DriverRequest {
            url: String::new(), // URL includes model and :generateContent / :streamGenerateContent
            method: "POST".into(),
            headers: HashMap::new(),
            body,
            stream: _stream,
        })
    }

    fn parse_response(&self, body: &Value) -> Result<DriverResponse, Error> {
        // Gemini: { candidates: [{ content: { parts: [{text: "..."}] }, finishReason }], usageMetadata }
        let content = body
            .pointer("/candidates/0/content/parts/0/text")
            .and_then(|v| v.as_str())
            .map(String::from);

        let finish_reason = body
            .pointer("/candidates/0/finishReason")
            .and_then(|v| v.as_str())
            .map(|r| match r {
                "STOP" => "stop".to_string(),
                "MAX_TOKENS" => "length".to_string(),
                "SAFETY" => "content_filter".to_string(),
                "RECITATION" => "content_filter".to_string(),
                other => other.to_lowercase(),
            });

        let usage = body.get("usageMetadata").map(|u| UsageInfo {
            prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0),
            completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0),
            total_tokens: u["totalTokenCount"].as_u64().unwrap_or(0),
        });

        // Gemini tool calls: functionCall parts
        let tool_calls: Vec<Value> = body
            .pointer("/candidates/0/content/parts")
            .and_then(|p| p.as_array())
            .map(|parts| {
                parts
                    .iter()
                    .filter(|p| p.get("functionCall").is_some())
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

        // Gemini streaming returns NDJSON — each line is a full generateContent response
        let v: Value = serde_json::from_str(data).map_err(|e| {
            Error::Protocol(ProtocolError::ValidationError(format!(
                "Failed to parse Gemini stream: {}",
                e
            )))
        })?;

        // Check for error
        if let Some(error) = v.get("error") {
            return Ok(Some(StreamingEvent::StreamError {
                error: error.clone(),
                event_id: None,
            }));
        }

        // Content delta
        if let Some(text) = v.pointer("/candidates/0/content/parts/0/text").and_then(|t| t.as_str())
        {
            if !text.is_empty() {
                return Ok(Some(StreamingEvent::PartialContentDelta {
                    content: text.to_string(),
                    sequence_id: None,
                }));
            }
        }

        // Finish reason
        if let Some(reason) = v
            .pointer("/candidates/0/finishReason")
            .and_then(|r| r.as_str())
        {
            if reason != "STOP" || v.pointer("/candidates/0/content/parts/0/text").is_none() {
                return Ok(Some(StreamingEvent::StreamEnd {
                    finish_reason: Some(match reason {
                        "STOP" => "stop".to_string(),
                        "MAX_TOKENS" => "length".to_string(),
                        other => other.to_lowercase(),
                    }),
                }));
            }
        }

        Ok(None)
    }

    fn supported_capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    fn is_stream_done(&self, _data: &str) -> bool {
        // Gemini uses NDJSON, stream ends when connection closes.
        // Individual chunks may contain finishReason but the stream itself
        // has no sentinel like OpenAI's [DONE].
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gemini_system_instruction() {
        let msgs = vec![
            Message::system("Be concise."),
            Message::user("Explain Rust."),
        ];
        let (sys, contents) = GeminiDriver::split_messages(&msgs);
        assert!(sys.is_some());
        assert_eq!(
            sys.unwrap()["parts"][0]["text"].as_str().unwrap(),
            "Be concise."
        );
        assert_eq!(contents.len(), 1);
        assert_eq!(contents[0]["role"], "user");
    }

    #[test]
    fn test_gemini_role_mapping() {
        let msgs = vec![
            Message::user("Hi"),
            Message::assistant("Hello!"),
            Message::user("How are you?"),
        ];
        let (_, contents) = GeminiDriver::split_messages(&msgs);
        assert_eq!(contents[0]["role"], "user");
        assert_eq!(contents[1]["role"], "model");
        assert_eq!(contents[2]["role"], "user");
    }

    #[test]
    fn test_gemini_build_request() {
        let driver = GeminiDriver::new("google", vec![Capability::Text]);
        let messages = vec![Message::user("Hello")];
        let req = driver
            .build_request(&messages, "gemini-2.0-flash", Some(0.5), Some(2048), false, None)
            .unwrap();
        assert_eq!(req.body["generationConfig"]["temperature"], 0.5);
        assert_eq!(req.body["generationConfig"]["maxOutputTokens"], 2048);
    }

    #[test]
    fn test_gemini_parse_response() {
        let driver = GeminiDriver::new("google", vec![]);
        let body = serde_json::json!({
            "candidates": [{
                "content": { "parts": [{"text": "Hi!"}], "role": "model" },
                "finishReason": "STOP"
            }],
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 3,
                "totalTokenCount": 8
            }
        });
        let resp = driver.parse_response(&body).unwrap();
        assert_eq!(resp.content.as_deref(), Some("Hi!"));
        assert_eq!(resp.finish_reason.as_deref(), Some("stop"));
        assert_eq!(resp.usage.unwrap().total_tokens, 8);
    }

    #[test]
    fn test_gemini_parse_stream_delta() {
        let driver = GeminiDriver::new("google", vec![]);
        let data = r#"{"candidates":[{"content":{"parts":[{"text":"World"}],"role":"model"}}]}"#;
        let event = driver.parse_stream_event(data).unwrap();
        match event {
            Some(StreamingEvent::PartialContentDelta { content, .. }) => {
                assert_eq!(content, "World");
            }
            _ => panic!("Expected PartialContentDelta"),
        }
    }

    #[test]
    fn test_gemini_finish_reason_normalization() {
        let driver = GeminiDriver::new("google", vec![]);
        let body = serde_json::json!({
            "candidates": [{
                "content": { "parts": [{"text": ""}], "role": "model" },
                "finishReason": "SAFETY"
            }]
        });
        let resp = driver.parse_response(&body).unwrap();
        assert_eq!(resp.finish_reason.as_deref(), Some("content_filter"));
    }
}
