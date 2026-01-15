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

## 🧩 Feature flags & re-exports

`ai-lib-rust` keeps the runtime core small, and exposes optional higher-level helpers behind feature flags.

For a deeper overview, see [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

- **Always available re-exports (crate root)**:
  - `AiClient`, `AiClientBuilder`, `CancelHandle`, `CallStats`, `ChatBatchRequest`, `EndpointExt`
  - `Message`, `MessageRole`, `StreamingEvent`, `ToolCall`
  - `Result<T>`, `Error`, `ErrorContext`
- **Feature-gated re-exports**:
  - **`routing_mvp`**: pure logic model management helpers (`CustomModelManager`, `ModelArray`, etc.)
  - **`interceptors`**: application-layer call hooks (`InterceptorPipeline`, `Interceptor`, `RequestContext`)

Enable with:

```toml
[dependencies]
ai-lib-rust = { version = "0.5.0", features = ["routing_mvp", "interceptors"] }
```

## 🗺️ Capability map (layered tools)

This is a structured view of what the crate provides, grouped by layers.

### 1) Protocol layer (`src/protocol/`)
- **`ProtocolLoader`**: load provider manifests from local paths / env paths / GitHub raw URLs
- **`ProtocolValidator`**: JSON Schema validation (supports offline via embedded schema)
- **`ProtocolManifest`**: typed representation of provider manifests
- **`UnifiedRequest`**: provider-agnostic request payload used by the runtime

### 2) Transport layer (`src/transport/`)
- **`HttpTransport`**: reqwest-based transport with proxy/timeout defaults and env knobs
- **API key resolution**: keyring → `<PROVIDER_ID>_API_KEY` env

### 3) Pipeline layer (`src/pipeline/`)
- **Operator pipeline**: decoder → selector → accumulator → fanout → event mapper
- **Streaming normalization**: maps provider frames to `StreamingEvent`

### 4) Client layer (`src/client/`)
- **`AiClient`**: runtime entry point; model-driven (`"provider/model"`)
- **Chat builder**: `client.chat().messages(...).stream().execute_stream()`
- **Batch**: `chat_batch`, `chat_batch_smart`
- **Observability**: `call_model_with_stats` returns `CallStats`
- **Cancellation**: `execute_stream_with_cancel()` → `CancelHandle`
- **Services**: `EndpointExt` for calling `services` declared in protocol manifests

### 5) Resilience layer (`src/resilience/` + `client/policy`)
- **Policy engine**: capability validation + retry/fallback decisions
- **Rate limiter**: token-bucket + adaptive header-driven mode
- **Circuit breaker**: minimal breaker with env or builder defaults
- **Backpressure**: max in-flight permit gating

### 6) Types layer (`src/types/`)
- **Messages**: `Message`, `MessageRole`, `MessageContent`, `ContentBlock`
- **Tools**: `ToolDefinition`, `FunctionDefinition`, `ToolCall`
- **Events**: `StreamingEvent`

### 7) Telemetry layer (`src/telemetry/`)
- **`FeedbackSink`** / **`FeedbackEvent`**: opt-in feedback reporting

### 8) Utils (`src/utils/`)
- JSONPath mapping helpers, tool-call assembler, and small runtime utilities

### 9) Optional helpers (feature-gated)
- **`routing_mvp`** (`src/routing/`): model selection + endpoint array load balancing (pure logic)
- **`interceptors`** (`src/interceptors/`): hooks around calls for logging/metrics/audit

## 🚀 Quick Start

### Basic Usage

```rust
use ai_lib_rust::{AiClient, Message};
use ai_lib_rust::types::events::StreamingEvent;
use futures::StreamExt;

#[tokio::main]
async fn main() -> ai_lib_rust::Result<()> {
    // Create client directly using provider/model string
    // This is fully protocol-driven and supports any provider defined in ai-protocol manifests
    let client = AiClient::new("anthropic/claude-3-5-sonnet").await?;

    let messages = vec![Message::user("Hello!")];

    // Streaming (unified events)
    let mut stream = client
        .chat()
        .messages(messages)
        .temperature(0.7)
        .stream()
        .execute_stream()
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
use ai_lib_rust::{Message, MessageRole};
use ai_lib_rust::types::message::{MessageContent, ContentBlock};

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
- `AI_LIB_BATCH_CONCURRENCY`: override concurrency limit for batch operations

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
ai-lib-rust = "0.5.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

## 🔧 Configuration

The library automatically looks for protocol files in the following locations (in order):

1. Custom path set via `ProtocolLoader::with_base_path()`
2. `ai-protocol/` subdirectory (Git submodule)
3. `../ai-protocol/` (sibling directory)
4. `../../ai-protocol/` (parent's sibling)

Protocol files should follow the AI-Protocol v1.5 specification structure. The runtime validates manifests against the official JSON Schema from the AI-Protocol repository.

## 🔐 Provider Requirements (API Keys)

Most providers require an API key. The runtime reads keys from (in order):

1. **OS Keyring** (optional, convenience feature)
   - **Windows**: Uses Windows Credential Manager
   - **macOS**: Uses Keychain
   - **Linux**: Uses Secret Service API
   - Service: `ai-protocol`, Username: provider id
   - **Note**: Keyring is optional and may not work in containers/WSL. Falls back to environment variables automatically.

2. **Environment Variable** (recommended for production)
   - Format: `<PROVIDER_ID>_API_KEY` (e.g. `DEEPSEEK_API_KEY`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`)
   - **Recommended for**: CI/CD, containers, WSL, production deployments

**Example**:
```bash
# Set API key via environment variable (recommended)
export DEEPSEEK_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."

# Or use keyring (optional, for local development)
# Windows: Stored in Credential Manager
# macOS: Stored in Keychain
```

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

- `basic_usage.rs`: Simple non-streaming chat completion
- `deepseek_chat_stream.rs`: Streaming chat example
- `deepseek_tool_call_stream.rs`: Tool calling with streaming
- `custom_protocol.rs`: Loading custom protocol configurations
- `list_models.rs`: Listing available models from provider
- `service_discovery.rs`: Service discovery and custom service calls

## 🧪 Testing

```bash
cargo test
```

## 📦 Batch (Chat)

For batch execution (order-preserving), use:

```rust
use ai_lib_rust::{AiClient, ChatBatchRequest, Message};

let client = AiClient::new("deepseek/deepseek-chat").await?;

let reqs = vec![
    ChatBatchRequest::new(vec![Message::user("Hello")]),
    ChatBatchRequest::new(vec![Message::user("Explain SSE in one sentence")])
        .temperature(0.2),
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

1. All protocol configurations follow the AI-Protocol v1.5 specification
2. New operators are properly documented
3. Tests are included for new features
4. Code follows Rust best practices and passes `cargo clippy`

## 📄 License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🔗 Related Projects

- [AI-Protocol](https://github.com/hiddenpath/ai-protocol): Protocol specification (v1.5)

---

**ai-lib-rust** - Where protocol meets performance. 🚀
