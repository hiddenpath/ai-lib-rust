# ai-lib-rust

**Protocol Runtime for AI-Protocol** - A high-performance Rust reference implementation.

`ai-lib-rust` is the Rust runtime implementation for the [AI-Protocol](https://github.com/hiddenpath/ai-protocol) specification. It embodies the core design principle: **一切逻辑皆算子，一切配置皆协议** (All logic is operators, all configuration is protocol).

## 🎯 Design Philosophy

Unlike traditional adapter libraries that hardcode provider-specific logic, `ai-lib-rust` is a **protocol-driven runtime** that executes AI-Protocol specifications. This means:

- **Zero hardcoded provider logic**: All behavior is driven by YAML protocol files
- **Operator-based architecture**: Processing is done through composable operators (Decoder → Selector → Accumulator → FanOut → EventMapper)
- **Hot-reloadable**: Protocol configurations can be updated without restarting the application
- **Unified interface**: Developers interact with a single, consistent API regardless of the underlying provider

## 🏗️ Architecture

The library is organized into three layers:

### 1. Protocol Specification Layer (`protocol/`)
- **Loader**: Loads protocol files from local filesystem, embedded assets, or remote URLs
- **Validator**: Validates protocols against JSON Schema
- **Schema**: Protocol structure definitions

### 2. Pipeline Interpreter Layer (`pipeline/`)
- **Decoder**: Parses raw bytes into protocol frames (SSE, JSON Lines, etc.)
- **Selector**: Filters frames using JSONPath expressions
- **Accumulator**: Accumulates stateful data (e.g., tool call arguments)
- **FanOut**: Handles multi-candidate scenarios
- **EventMapper**: Converts protocol frames to unified events

### 3. User Interface Layer (`client/`, `types/`)
- **Client**: Unified client interface
- **Types**: Standard type system based on AI-Protocol `standard_schema`

## 🚀 Quick Start

### Basic Usage (developer-friendly facade)

```rust
use ai_lib_rust::prelude::*;
use futures::StreamExt;

#[tokio::main]
async fn main() -> ai_lib_rust::Result<()> {
    // Create client for a specific model.
    // Note: providers typically require an API key via env var, e.g. ANTHROPIC_API_KEY / OPENAI_API_KEY / DEEPSEEK_API_KEY.
    let client = Provider::Anthropic.model("claude-3-5-sonnet").build_client().await?;

    let messages = vec![Message::user("Hello!")];

    // Streaming (unified events)
    let mut stream = client
        .chat_completion_stream(ChatCompletionRequest::new(messages).temperature(0.7).stream())
        .await?;

    while let Some(event) = stream.next().await {
        match event? {
            StreamingEvent::PartialContentDelta { content, .. } => print!("{content}"),
            StreamingEvent::StreamEnd { .. } => break,
            _ => {}
        }
    }

    Ok(())
}
```

### Multimodal (Image / Audio)

Multimodal inputs are represented as `MessageContent::Blocks(Vec<ContentBlock>)`.

```rust
use ai_lib_rust::prelude::*;

fn multimodal_message(image_path: &str) -> ai_lib_rust::Result<Message> {
    let blocks = vec![
        ContentBlock::text("Describe this image briefly."),
        ContentBlock::image_from_file(image_path)?,
    ];
    Ok(Message::with_content(
        MessageRole::User,
        MessageContent::blocks(blocks),
    ))
}
```

### Useful environment variables

- `AI_PROTOCOL_DIR` / `AI_PROTOCOL_PATH`: path to your local `ai-protocol` repo root (containing `v1/`)
- `AI_LIB_ATTEMPT_TIMEOUT_MS`: per-attempt timeout guard used by the unified policy engine
- `AI_LIB_DEFAULT_MODEL_<PROVIDER>`: optional default model name for `client_from_provider(Provider::X)`

### Custom Protocol

```rust
use ai_lib_rust::protocol::ProtocolLoader;

let loader = ProtocolLoader::new()
    .with_base_path("./ai-protocol")
    .with_hot_reload(true);

let manifest = loader.load_provider("openai").await?;
```

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ai-lib-rust = "0.2"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

## 🔧 Configuration

The library automatically looks for protocol files in the following locations (in order):

1. Custom path set via `ProtocolLoader::with_base_path()`
2. `ai-protocol/` subdirectory (Git submodule)
3. `../ai-protocol/` (sibling directory)
4. `../../ai-protocol/` (parent's sibling)

Protocol files should follow the AI-Protocol v1.1 specification structure.

## 🔐 Provider Requirements (API Keys)

Most providers require an API key. The runtime reads keys from:
- OS keyring (service: `ai-protocol`, username: provider id)
- Environment variable: `<PROVIDER_ID>_API_KEY` (e.g. `DEEPSEEK_API_KEY`)

Provider-specific details vary, but `ai-lib-rust` normalizes them behind a unified client API.

## 🌐 Proxy / Timeout / Backpressure (Production knobs)

- **Proxy**: set `AI_PROXY_URL` (e.g. `http://user:pass@host:port`)
- **HTTP timeout**: set `AI_HTTP_TIMEOUT_SECS` (fallback: `AI_TIMEOUT_SECS`)
- **In-flight limit**: set `AI_LIB_MAX_INFLIGHT` or use `AiClientBuilder::max_inflight(n)`
- **Rate limiting** (optional): set either
  - `AI_LIB_RPS` (requests per second), or
  - `AI_LIB_RPM` (requests per minute)
- **Circuit breaker** (optional): enable via `AiClientBuilder::circuit_breaker_default()` or env
  - `AI_LIB_BREAKER_FAILURE_THRESHOLD` (default 5)
  - `AI_LIB_BREAKER_COOLDOWN_SECS` (default 30)

## 📊 Observability: CallStats

If you need per-call stats (latency, retries, request ids, endpoint), use:

```rust
let (resp, stats) = client.call_model_with_stats(unified_req).await?;
println!("client_request_id={}", stats.client_request_id);
```

## 🛑 Cancellable Streaming

```rust
let (mut stream, cancel) = client.chat().messages(messages).stream().execute_stream_with_cancel().await?;
// cancel.cancel(); // emits StreamEnd{finish_reason:"cancelled"}, drops the underlying network stream, and releases inflight permit
```

## 🧾 Optional Feedback (Choice Selection)

Telemetry is **opt-in**. You can inject a `FeedbackSink` and report feedback explicitly:

```rust
use ai_lib_rust::telemetry::{FeedbackEvent, ChoiceSelectionFeedback};

client.report_feedback(FeedbackEvent::ChoiceSelection(ChoiceSelectionFeedback {
    request_id: stats.client_request_id.clone(),
    chosen_index: 0,
    rejected_indices: None,
    latency_to_select_ms: None,
    ui_context: None,
    candidate_hashes: None,
})).await?;
```

## 🎨 Key Features

### Protocol-Driven Architecture

No `match provider` statements. All logic is derived from protocol configuration:

```rust
// The pipeline is built dynamically from protocol manifest
let pipeline = Pipeline::from_manifest(&manifest)?;

// Operators are configured via YAML, not hardcoded
// Adding a new provider requires zero code changes
```

### Multi-Candidate Support

Automatically handles multi-candidate scenarios through the `FanOut` operator:

```yaml
streaming:
  candidate:
    candidate_id_path: "$.choices[*].index"
    fan_out: true
```

### Tool Accumulation

Stateful accumulation of tool call arguments:

```yaml
streaming:
  accumulator:
    stateful_tool_parsing: true
    key_path: "$.delta.partial_json"
    flush_on: "$.type == 'content_block_stop'"
```

### Hot Reload

Protocol configurations can be updated at runtime:

```rust
let loader = ProtocolLoader::new().with_hot_reload(true);
// Protocol changes are automatically picked up
```

## 📚 Examples

See the `examples/` directory:

- `basic_usage.rs`: Simple chat completion example
- `custom_protocol.rs`: Loading custom protocol configurations

## 🧪 Testing

```bash
cargo test
```

## 📦 Batch (Chat)

For batch execution (order-preserving), use:

```rust
use ai_lib_rust::{ChatBatchRequest, Message, MessageRole};
use ai_lib_rust::types::message::MessageContent;

let reqs = vec![
    ChatBatchRequest::new(vec![Message {
        role: MessageRole::User,
        content: MessageContent::Text("Hello".to_string()),
    }]),
    ChatBatchRequest::new(vec![Message {
        role: MessageRole::User,
        content: MessageContent::Text("Explain SSE in one sentence".to_string()),
    }]).temperature(0.2),
];

let results = client.chat_batch(reqs, Some(5)).await;
```

### Smart batch tuning

If you prefer a conservative default heuristic, use:

```rust
let results = client.chat_batch_smart(reqs).await;
```

Override concurrency with:
- `AI_LIB_BATCH_CONCURRENCY`

## 🤝 Contributing

Contributions are welcome! Please ensure that:

1. All protocol configurations follow the AI-Protocol v1.1 specification
2. New operators are properly documented
3. Tests are included for new features

## 📄 License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🔗 Related Projects

- [AI-Protocol](https://github.com/hiddenpath/ai-protocol): Protocol specification
- [ai-lib](https://github.com/hiddenpath/ai-lib): Original Rust implementation (being migrated)

---

**ai-lib-rust** - Where protocol meets performance. 🚀
