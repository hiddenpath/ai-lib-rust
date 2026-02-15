//! Benchmarks for request compilation performance
//!
//! This benchmark measures:
//! - UnifiedRequest to provider-specific format compilation
//! - Parameter mapping overhead
//! - Message serialization speed

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

use ai_lib_rust::protocol::{ProtocolManifest, UnifiedRequest};
use ai_lib_rust::types::message::{Message, MessageContent, MessageRole};
use ai_lib_rust::types::tool::ToolDefinition;

const SAMPLE_PROTOCOL_JSON: &str = r#"{
  "$schema": "https://raw.githubusercontent.com/hiddenpath/ai-protocol/main/schemas/v1.json",
  "id": "benchmark-provider",
  "protocol_version": "1.5",
  "name": "Benchmark Provider",
  "status": "stable",
  "category": "ai_provider",
  "official_url": "https://example.com",
  "support_contact": "support@example.com",
  "endpoint": {
    "base_url": "https://api.example.com",
    "timeout_ms": 30000
  },
  "availability": {
    "required": true,
    "regions": ["global"],
    "check": {
      "method": "HEAD",
      "path": "/health",
      "expected_status": [200]
    }
  },
  "capabilities": {
    "streaming": true,
    "tools": true,
    "vision": true
  },
  "auth": {
    "type": "bearer",
    "token_env": "EXAMPLE_API_KEY"
  },
  "parameter_mappings": {
    "model": "model",
    "messages": "messages",
    "temperature": "temperature",
    "max_tokens": "max_tokens",
    "stream": "stream",
    "tools": "tools",
    "tool_choice": "tool_choice"
  },
  "streaming": {
    "decoder": {
      "format": "sse",
      "strategy": "openai_sse"
    },
    "event_map": []
  }
}"#;

fn create_simple_request() -> UnifiedRequest {
    UnifiedRequest {
        operation: "chat".to_string(),
        model: "gpt-4o".to_string(),
        messages: vec![Message {
            role: MessageRole::User,
            content: MessageContent::Text("Hello, world!".to_string()),
        }],
        temperature: Some(0.7),
        max_tokens: Some(1000),
        stream: true,
        tools: None,
        tool_choice: None,
        response_format: None,
    }
}

fn create_complex_request() -> UnifiedRequest {
    let tool = ToolDefinition {
        tool_type: "function".to_string(),
        function: ai_lib_rust::types::tool::FunctionDefinition {
            name: "get_weather".to_string(),
            description: Some("Get the current weather in a given location".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state"
                    },
                    "unit": {
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"]
                    }
                },
                "required": ["location"]
            })),
        },
    };

    UnifiedRequest {
        operation: "chat".to_string(),
        model: "gpt-4o".to_string(),
        messages: vec![
            Message {
                role: MessageRole::System,
                content: MessageContent::Text(
                    "You are a helpful assistant that can check the weather.".to_string(),
                ),
            },
            Message {
                role: MessageRole::User,
                content: MessageContent::Text("What is the weather like in Tokyo?".to_string()),
            },
        ],
        temperature: Some(0.7),
        max_tokens: Some(2000),
        stream: true,
        tools: Some(vec![tool]),
        tool_choice: Some(serde_json::json!("auto")),
        response_format: None,
    }
}

fn create_long_conversation() -> UnifiedRequest {
    let mut messages = vec![Message {
        role: MessageRole::System,
        content: MessageContent::Text("You are a helpful assistant.".to_string()),
    }];

    for i in 0..50 {
        messages.push(Message {
            role: MessageRole::User,
            content: MessageContent::Text(format!("User message number {}", i)),
        });
        messages.push(Message {
            role: MessageRole::Assistant,
            content: MessageContent::Text(format!("Assistant response number {}", i)),
        });
    }

    UnifiedRequest {
        operation: "chat".to_string(),
        model: "gpt-4o".to_string(),
        messages,
        temperature: Some(0.7),
        max_tokens: Some(4000),
        stream: true,
        tools: None,
        tool_choice: None,
        response_format: None,
    }
}

fn bench_request_compilation(c: &mut Criterion) {
    let manifest: ProtocolManifest = serde_json::from_str(SAMPLE_PROTOCOL_JSON).unwrap();

    let mut group = c.benchmark_group("request_compilation");

    let simple_request = create_simple_request();
    let complex_request = create_complex_request();
    let long_request = create_long_conversation();

    group.bench_with_input(
        BenchmarkId::new("compile", "simple"),
        &simple_request,
        |b, req| {
            b.iter(|| manifest.compile_request(black_box(req)).unwrap())
        },
    );

    group.bench_with_input(
        BenchmarkId::new("compile", "with_tools"),
        &complex_request,
        |b, req| {
            b.iter(|| manifest.compile_request(black_box(req)).unwrap())
        },
    );

    group.bench_with_input(
        BenchmarkId::new("compile", "long_conversation"),
        &long_request,
        |b, req| {
            b.iter(|| manifest.compile_request(black_box(req)).unwrap())
        },
    );

    group.finish();
}

fn bench_message_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("message_serialization");

    let messages: Vec<Message> = (0..10)
        .map(|i| Message {
            role: if i % 2 == 0 {
                MessageRole::User
            } else {
                MessageRole::Assistant
            },
            content: MessageContent::Text(format!("Message content number {}", i)),
        })
        .collect();

    group.throughput(Throughput::Elements(messages.len() as u64));

    group.bench_function("serialize_messages", |b| {
        b.iter(|| {
            let _: Vec<serde_json::Value> = black_box(&messages)
                .iter()
                .map(|m| serde_json::to_value(m).unwrap())
                .collect();
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_request_compilation,
    bench_message_serialization,
);
criterion_main!(benches);
