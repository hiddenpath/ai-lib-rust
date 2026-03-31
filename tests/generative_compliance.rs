//! Generative LLM alignment tests (gen-001 … gen-007) matching ai-protocol compliance intent.
//!
//! 中文：与 Python `test_generative.py` 及 compliance `08-generative-capabilities` 语义对齐。

use ai_lib_rust::client::{classify_error_from_response, PolicyEngine};
use ai_lib_rust::drivers::AnthropicDriver;
use ai_lib_rust::drivers::{OpenAiDriver, ProviderDriver};
use ai_lib_rust::error_code::StandardErrorCode;
use ai_lib_rust::pipeline::event_map::PathEventMapper;
use ai_lib_rust::pipeline::Mapper;
use ai_lib_rust::protocol::config::Capabilities;
use ai_lib_rust::protocol::{ProtocolManifest, UnifiedRequest};
use ai_lib_rust::structured::JsonModeConfig;
use ai_lib_rust::types::events::StreamingEvent;
use ai_lib_rust::types::message::Message;
use ai_lib_rust::types::tool::{FunctionDefinition, ToolDefinition};
use ai_lib_rust::Error;
use futures::StreamExt;
use serde_json::json;

#[test]
fn gen001_v2_capability_flags_parse() {
    let caps: Capabilities = serde_yaml::from_str(
        r"
required: [text, streaming, tools]
optional: [reasoning, structured_output]
feature_flags:
  structured_output: true
  extended_thinking: true
",
    )
    .expect("capabilities");
    assert!(caps.streaming);
    assert!(caps.tools);
    assert!(caps.reasoning);
    assert!(caps.structured_output);
}

#[test]
fn gen003_compile_request_merges_response_format_when_allowed() {
    let manifest: ProtocolManifest = serde_yaml::from_str(
        r#"
id: gen003
protocol_version: "1.5"
status: stable
category: ai_provider
official_url: "https://example.com"
support_contact: "https://example.com/support"
endpoint:
  base_url: "https://api.example.com"
capabilities:
  required: [text, streaming, structured_output]
  optional: []
  feature_flags:
    structured_output: true
parameter_mappings:
  model: model
  messages: messages
  stream: stream
"#,
    )
    .expect("manifest");

    let req = UnifiedRequest {
        model: "gpt-4o".into(),
        operation: "chat".into(),
        messages: vec![Message::user("List three colors as JSON")],
        stream: false,
        response_format: Some(JsonModeConfig::json_object()),
        ..Default::default()
    };

    let body = manifest.compile_request(&req).expect("compile");
    assert_eq!(body["response_format"]["type"], json!("json_object"));
}

#[test]
fn gen003_policy_rejects_response_format_without_structured_capability() {
    let manifest: ProtocolManifest = serde_yaml::from_str(
        r#"
id: no-json
protocol_version: "1.5"
status: stable
category: ai_provider
official_url: "https://example.com"
support_contact: "s"
endpoint:
  base_url: "https://api.example.com"
capabilities:
  required: [text, streaming]
  optional: []
parameter_mappings: {}
"#,
    )
    .expect("manifest");
    let policy = PolicyEngine::new(&manifest);
    let req = UnifiedRequest {
        response_format: Some(JsonModeConfig::json_object()),
        ..Default::default()
    };
    assert!(policy.validate_capabilities(&req).is_err());
}

#[test]
fn gen005_context_length_exceeded_maps_to_e1005() {
    let body = json!({
        "error": {
            "message": "This model's maximum context length is 128000 tokens.",
            "type": "invalid_request_error",
            "code": "context_length_exceeded"
        }
    });
    let y = serde_yaml::to_value(body).expect("yaml");
    let code = classify_error_from_response(400, Some(&y));
    assert_eq!(code, StandardErrorCode::RequestTooLarge);
    assert_eq!(code.code(), "E1005");
}

#[tokio::test]
async fn gen004_path_mapper_tool_deltas_and_usage() {
    let mapper = PathEventMapper::new(None, None, None, None);
    let frames = vec![
        json!({"choices":[{"delta":{"tool_calls":[{"index":0,"id":"call_abc","type":"function","function":{"name":"get_weather","arguments":""}}]}}]}),
        json!({"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"loc"}}]}}]}),
        json!({"choices":[{"delta":{"tool_calls":[{"index":0,"function":{"arguments":"ation\": \"SF\"}"}}]}}]}),
        json!({"choices":[{"delta":{},"finish_reason":"tool_calls"}],"usage":{"prompt_tokens":20,"completion_tokens":15,"total_tokens":35}}),
    ];
    let input = Box::pin(futures::stream::iter(
        frames.into_iter().map(Ok::<_, Error>),
    ));
    let mut stream = mapper.map(input).await.expect("mapper");

    let mut arg_chunks: Vec<String> = Vec::new();
    let mut usage_val = None;
    while let Some(ev) = stream.next().await {
        match ev.expect("event") {
            StreamingEvent::PartialToolCall { arguments, .. } => arg_chunks.push(arguments),
            StreamingEvent::Metadata { usage, .. } => usage_val = usage,
            _ => {}
        }
    }
    let merged: String = arg_chunks.into_iter().collect();
    assert!(
        merged.contains("location") && merged.contains("SF"),
        "args={merged}"
    );
    let u = usage_val.expect("usage");
    assert_eq!(u.get("prompt_tokens").and_then(|x| x.as_u64()), Some(20));
}

#[test]
fn gen006_anthropic_thinking_delta_from_driver() {
    let driver = AnthropicDriver::new("anthropic", vec![]);
    let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"Let me analyze..."}}"#;
    let ev = driver.parse_stream_event(data).expect("parse");
    match ev {
        Some(StreamingEvent::ThinkingDelta { thinking, .. }) => {
            assert!(thinking.contains("Let me analyze"));
        }
        other => panic!("expected ThinkingDelta, got {:?}", other),
    }
}

#[tokio::test]
async fn gen006_openai_reasoning_path_mapper() {
    let mapper = PathEventMapper::new(None, None, None, None);
    let frame = json!({"choices":[{"index":0,"delta":{"reasoning_content":"Let me think..."}}]});
    let input = Box::pin(futures::stream::iter(std::iter::once(Ok::<_, Error>(
        frame,
    ))));
    let mut s = mapper.map(input).await.expect("map");
    let first = s.next().await.expect("one event").expect("ok");
    match first {
        StreamingEvent::ThinkingDelta { thinking, .. } => assert_eq!(thinking, "Let me think..."),
        e => panic!("expected ThinkingDelta, got {:?}", e),
    }
}

#[test]
fn gen007_mcp_tool_type_rejected_without_mcp_client_capability() {
    let manifest: ProtocolManifest = serde_yaml::from_str(
        r#"
id: no-mcp
protocol_version: "1.5"
status: stable
category: ai_provider
official_url: "https://example.com"
support_contact: "s"
endpoint:
  base_url: "https://api.example.com"
capabilities:
  required: [text, streaming, tools]
  optional: []
parameter_mappings: {}
"#,
    )
    .expect("manifest");
    let policy = PolicyEngine::new(&manifest);
    let req = UnifiedRequest {
        tools: Some(vec![ToolDefinition {
            tool_type: "mcp".into(),
            function: FunctionDefinition {
                name: "list_tools".into(),
                description: None,
                parameters: None,
            },
        }]),
        ..Default::default()
    };
    let err = policy.validate_capabilities(&req).expect_err("should fail");
    assert_eq!(
        err.standard_code(),
        Some(StandardErrorCode::RequestTooLarge)
    );
}

#[test]
fn gen007_mcp_namespaced_function_allowed_when_mcp_client_declared() {
    let manifest: ProtocolManifest = serde_yaml::from_str(
        r#"
id: with-mcp
protocol_version: "1.5"
status: stable
category: ai_provider
official_url: "https://example.com"
support_contact: "s"
endpoint:
  base_url: "https://api.example.com"
capabilities:
  required: [text, streaming, tools]
  optional: [mcp_client]
parameter_mappings: {}
"#,
    )
    .expect("manifest");
    assert!(manifest.supports_capability("mcp_client"));
    let policy = PolicyEngine::new(&manifest);
    let req = UnifiedRequest {
        tools: Some(vec![ToolDefinition {
            tool_type: "function".into(),
            function: FunctionDefinition {
                name: "mcp__srv__echo".into(),
                description: None,
                parameters: None,
            },
        }]),
        ..Default::default()
    };
    policy.validate_capabilities(&req).expect("mcp bridge ok");
}

#[test]
fn gen002_openai_usage_reasoning_tokens_on_driver() {
    let driver = OpenAiDriver::new("openai", vec![]);
    let body = json!({
        "choices": [{"message": {"content": "Hello, world!"}, "finish_reason": "stop"}],
        "usage": {
            "prompt_tokens": 10,
            "completion_tokens": 5,
            "total_tokens": 15,
            "completion_tokens_details": {"reasoning_tokens": 3}
        }
    });
    let resp = driver.parse_response(&body).expect("parse");
    let u = resp.usage.expect("usage");
    assert_eq!(u.reasoning_tokens, Some(3));
}
