# ai-lib-rust

**Protocol Runtime for AI-Protocol** - A high-performance Rust reference implementation.

`ai-lib-rust` is the Rust runtime implementation for the [AI-Protocol](https://github.com/ailib-official/ai-protocol) specification. It embodies the core design principle: **一切逻辑皆算子，一切配置皆协议** (All logic is operators, all configuration is protocol).

## 🎯 Design Philosophy

Unlike traditional adapter libraries that hardcode provider-specific logic, `ai-lib-rust` is a **protocol-driven runtime** that executes AI-Protocol specifications. This means:

- **Zero hardcoded provider logic**: All behavior is driven by protocol manifests (source YAML or dist JSON)
- **Operator-based architecture**: Processing is done through composable operators (Decoder → Selector → Accumulator → FanOut → EventMapper)
- **Hot-reloadable**: Protocol configurations can be updated without restarting the application
- **Unified interface**: Developers interact with a single, consistent API regardless of the underlying provider

## 🏗️ Cargo workspace

The repo is a **Cargo workspace** with four published-style crates plus an optional wasmtime harness:

| Crate | Path | Role |
|-------|------|------|
| `ai-lib-core` | `crates/ai-lib-core` | Execution layer: client, drivers, pipeline, protocol, types, transport, structured output, etc. |
| `ai-lib-contact` | `crates/ai-lib-contact` | Policy layer: cache, batch, routing, plugins, interceptors, tokens, telemetry, guardrails, resilience (depends on `ai-lib-core`). |
| `ai-lib-wasm` | `crates/ai-lib-wasm` | WASI thin exports over `ai-lib-core` for `wasm32-wasip1` (6 host-facing functions, < 2 MB). Not published to crates.io. |
| `ai-lib-rust` | `crates/ai-lib-rust` | Thin facade: re-exports core + contact so existing `ai_lib_rust::…` paths stay stable. Holds integration tests, examples, and CLI bins. |
| `ai-lib-wasmtime-harness` | `crates/ai-lib-wasmtime-harness` | PT-073: optional integration tests that load `ai_lib_wasm.wasm` in wasmtime (heavy deps; run with `-p ai-lib-wasmtime-harness`). |

From the repo root, `cargo test` runs the default workspace members (the facade crate and its tests). Depend on `ai-lib-core` or `ai-lib-contact` directly if you want a smaller dependency surface without the full umbrella crate.

### WASM target

```bash
# Build for server-side WASM (wasmtime, etc.)
cargo build -p ai-lib-wasm --target wasm32-wasip1 --release
# Binary lands at target/wasm32-wasip1/release/ai_lib_wasm.wasm (~1.2 MB)
```

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

## 🔄 V2 Protocol Alignment

Starting with v0.7.0, `ai-lib-rust` aligns with the **AI-Protocol V2** specification. V0.8.0 adds full V2 runtime support including V2 manifest parsing, provider drivers, MCP, Computer Use, and extended multimodal.

### Standard Error Codes (V2)

All provider errors are classified into 13 standard error codes with unified retry/fallback semantics:

| Code | Name | Retryable | Fallbackable |
|------|------|-----------|--------------|
| E1001 | `invalid_request` | No | No |
| E1002 | `authentication` | No | Yes |
| E1003 | `permission_denied` | No | No |
| E1004 | `not_found` | No | No |
| E1005 | `request_too_large` | No | No |
| E2001 | `rate_limited` | Yes | Yes |
| E2002 | `quota_exhausted` | No | Yes |
| E3001 | `server_error` | Yes | Yes |
| E3002 | `overloaded` | Yes | Yes |
| E3003 | `timeout` | Yes | Yes |
| E4001 | `conflict` | Yes | No |
| E4002 | `cancelled` | No | No |
| E9999 | `unknown` | No | No |

Classification follows a priority pipeline: provider-specific error code → HTTP status override → standard HTTP mapping → `E9999`.

### Compliance Tests

Cross-runtime behavioral consistency is verified by a shared YAML-based test suite from the `ai-protocol` repository:

```bash
# Run compliance tests (facade crate; shared YAML runner)
cargo test --test compliance

# With explicit compliance directory
COMPLIANCE_DIR=../ai-protocol/tests/compliance cargo test --test compliance

# Same full YAML suite from ai-lib-core (shared `compliance_runner` module)
COMPLIANCE_DIR=../ai-protocol/tests/compliance cargo test -p ai-lib-core --test compliance_from_core

# PT-073: wasmtime loads release WASI build (after `cargo build -p ai-lib-wasm --target wasm32-wasip1 --release`)
cargo test -p ai-lib-wasmtime-harness --test wasm_compliance
```

For details, see [CROSS_RUNTIME.md](https://github.com/ailib-official/ai-protocol/blob/main/docs/CROSS_RUNTIME.md).

### Testing with ai-protocol-mock

For integration and MCP tests without real API calls, use [ai-protocol-mock](https://github.com/ailib-official/ai-protocol-mock):

```bash
# Start mock server (from ai-protocol-mock repo)
docker-compose up -d

# Run tests with mock
MOCK_HTTP_URL=http://localhost:4010 MOCK_MCP_URL=http://localhost:4010/mcp cargo test -- --ignored --nocapture

# Run specific mock integration tests
MOCK_HTTP_URL=http://localhost:4010 cargo test test_sse_streaming_via_mock test_error_classification_via_mock -- --ignored --nocapture
```

Or in code: `AiClientBuilder::new().base_url_override("http://localhost:4010").build(...)`

## 🧩 Feature flags & re-exports

`ai-lib-rust` keeps the runtime core small, and exposes optional capabilities behind feature flags. This aligns with the V2 "lean core, progressive complexity" design principle.

For a deeper overview, see [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md).

- **Always available re-exports (crate root)**:
  - `AiClient`, `AiClientBuilder`, `CancelHandle`, `CallStats`, `ChatBatchRequest`, `ClientMetrics`, `EndpointExt`
  - `Message`, `MessageRole`, `StreamingEvent`, `ToolCall`
  - `Result<T>`, `Error`, `ErrorContext`
  - `FeedbackEvent`, `FeedbackSink` (core feedback types)
- **Capability features (V2 aligned)**:
  - **`embeddings`**: embedding generation (`EmbeddingClient`)
  - **`batch`**: batch API processing (`BatchExecutor`)
  - **`guardrails`**: input/output validation
  - **`tokens`**: token counting and cost estimation
  - **`telemetry`**: advanced observability sinks (`InMemoryFeedbackSink`, `ConsoleFeedbackSink`, etc.)
  - **`mcp`**: MCP (Model Context Protocol) tool bridge — namespace-based tool conversion and filtering
  - **`computer_use`**: Computer Use abstraction — safety policies, domain allowlists, action validation
  - **`multimodal`**: Extended multimodal support — vision, audio, video modality validation and format checks
  - **`reasoning`**: Extended reasoning / chain-of-thought support
- **Infrastructure features**:
  - **`routing_mvp`**: pure logic model management helpers (`CustomModelManager`, `ModelArray`, etc.)
  - **`interceptors`**: application-layer call hooks (`InterceptorPipeline`, `Interceptor`, `RequestContext`)
- **Meta-feature**:
  - **`full`**: enables all capability and infrastructure features

Enable with:

```toml
[dependencies]
# Lean core (default)
ai-lib-rust = "0.8.0"

# With specific capabilities
ai-lib-rust = { version = "0.8.0", features = ["embeddings", "telemetry"] }

# Everything enabled
ai-lib-rust = { version = "0.8.0", features = ["full"] }
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
- **Extended feedback types**: `RatingFeedback`, `ThumbsFeedback`, `TextFeedback`, `CorrectionFeedback`, `RegenerateFeedback`, `StopFeedback`
- **Multiple sinks**: `InMemoryFeedbackSink`, `ConsoleFeedbackSink`, `CompositeFeedbackSink`
- **Global sink management**: `get_feedback_sink()`, `set_feedback_sink()`, `report_feedback()`

### 8) Embedding layer (`src/embeddings/`) - NEW in v0.6.5
- **`EmbeddingClient`** / **`EmbeddingClientBuilder`**: Generate embeddings from text
- **Types**: `Embedding`, `EmbeddingRequest`, `EmbeddingResponse`, `EmbeddingUsage`
- **Vector operations**: `cosine_similarity`, `dot_product`, `euclidean_distance`, `manhattan_distance`
- **Utilities**: `normalize_vector`, `average_vectors`, `weighted_average_vectors`, `find_most_similar`

### 9) Cache layer (`src/cache/`) - NEW in v0.6.5
- **`CacheBackend`** trait with `MemoryCache` and `NullCache` implementations
- **`CacheManager`**: TTL-based caching with statistics
- **`CacheKey`** / **`CacheKeyGenerator`**: Deterministic cache key generation

### 10) Token layer (`src/tokens/`) - NEW in v0.6.5
- **`TokenCounter`** trait: `CharacterEstimator`, `AnthropicEstimator`, `CachingCounter`
- **`ModelPricing`**: Pre-configured pricing for GPT-4o, Claude models
- **`CostEstimate`**: Calculate request costs

### 11) Batch layer (`src/batch/`) - NEW in v0.6.5
- **`BatchCollector`** / **`BatchConfig`**: Accumulate requests for batch processing
- **`BatchExecutor`**: Execute batches with configurable strategies
- **`BatchResult`**: Structured batch execution results

### 12) Plugin layer (`src/plugins/`) - NEW in v0.6.5
- **`Plugin`** trait with lifecycle hooks
- **`PluginRegistry`**: Centralized plugin management
- **Hook system**: `HookType`, `Hook`, `HookManager`
- **Middleware**: `Middleware`, `MiddlewareChain` for request/response transformation

### 13) Utils (`src/utils/`)
- JSONPath mapping helpers, tool-call assembler, and small runtime utilities

### 14) Optional helpers (feature-gated)
- **`routing_mvp`** (`src/routing/`): model selection + endpoint array load balancing (pure logic)
- **`interceptors`** (`src/interceptors/`): hooks around calls for logging/metrics/audit

## 🚀 Quick Start

### Sharing the client across tasks

`AiClient` does not implement `Clone` (by design, for API key and provider ToS compliance).
Use `Arc<AiClient>` to share across async tasks:

```rust
use ai_lib_rust::{AiClient, Message};
use std::sync::Arc;

#[tokio::main]
async fn main() -> ai_lib_rust::Result<()> {
    let client = Arc::new(AiClient::new("openai/gpt-4o").await?);
    // Pass Arc::clone(&client) to spawned tasks
    let handle = tokio::spawn({
        let c = Arc::clone(&client);
        async move { c.chat().messages(vec![Message::user("Hi")]).execute().await }
    });
    let _ = handle.await?;
    Ok(())
}
```

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
ai-lib-rust = "0.8.0"
tokio = { version = "1.0", features = ["full"] }
futures = "0.3"
```

## 🔧 Configuration

The library automatically looks for protocol manifests in the following locations (in order):

1. Custom path set via `ProtocolLoader::with_base_path()`
2. `AI_PROTOCOL_DIR` / `AI_PROTOCOL_PATH` (local path or GitHub raw URL)
3. Common dev paths: `ai-protocol/`, `../ai-protocol/`, `../../ai-protocol/`
4. Last resort: GitHub raw `hiddenpath/ai-protocol` (main)

For each base path, provider manifests are resolved in a backward-compatible order:
`dist/v1/providers/<id>.json` → `v1/providers/<id>.yaml`.

Protocol manifests should follow the AI-Protocol v1.5 specification structure. The runtime validates manifests against the official JSON Schema from the AI-Protocol repository.

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

// Operators are configured via manifests (YAML/JSON), not hardcoded
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
- `test_protocol_loading.rs`: Protocol loading sanity check

## 🧪 Testing

```bash
# Run all tests
cargo test

# Run compliance tests (cross-runtime consistency)
cargo test --test compliance

# Run with all features enabled
cargo test --features full
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

1. All protocol configurations follow the AI-Protocol specification (v1.5 / V2)
2. New operators are properly documented
3. Tests are included for new features
4. Compliance tests pass for cross-runtime behaviors (`cargo test --test compliance`)
5. Code follows Rust best practices and passes `cargo clippy`

## 📄 License

This project is licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT License ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🔗 Related Projects

- [AI-Protocol](https://github.com/ailib-official/ai-protocol): Protocol specification (v1.5 / V2)
- [ai-lib-python](https://github.com/ailib-official/ai-lib-python): Python runtime implementation

---

**ai-lib-rust** - Where protocol meets performance. 🚀
