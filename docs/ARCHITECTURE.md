# Architecture: ai-lib-rust (AI-Protocol Runtime)

This document describes the **runtime-first** architecture of `ai-lib-rust`, a protocol-driven Rust implementation for [AI-Protocol](https://github.com/hiddenpath/ai-protocol).

Core principle:

> **All logic is operators, all configuration is protocol.**

The crate avoids hardcoding provider-specific behavior. Instead, it **loads AI-Protocol manifests** (YAML), validates them, and executes requests through a **composable operator pipeline**.

---

## 1) High-level module map

`ai-lib-rust` is organized into layers (from “most stable” at the bottom to “most user-facing” at the top):

- **Protocol layer**: `src/protocol/`
- **Transport layer**: `src/transport/`
- **Pipeline layer (operators)**: `src/pipeline/`
- **Client layer (API)**: `src/client/`
- **Resilience layer (controls)**: `src/resilience/` and `src/client/policy`
- **Types layer (standard schema types)**: `src/types/`
- **Telemetry layer**: `src/telemetry/`
- **Utilities**: `src/utils/`
- **New in v0.6.5** (features from ai-lib-python):
  - **Embeddings layer**: `src/embeddings/` - Embedding client and vector operations
  - **Cache layer**: `src/cache/` - Response caching with TTL support
  - **Tokens layer**: `src/tokens/` - Token counting and cost estimation
  - **Batch layer**: `src/batch/` - Request batching and execution
  - **Plugins layer**: `src/plugins/` - Plugin system with hooks and middleware
- **Optional helpers (feature-gated)**:
  - `routing_mvp`: `src/routing/`
  - `interceptors`: `src/interceptors/`

At the crate root, we **re-export** the most common entry points to keep imports short (see `src/lib.rs`).

---

## 2) Runtime data flow (request → stream of events)

### 2.1 Manifest-driven runtime construction

At startup (or on demand), the runtime resolves a **model id** like:

- `provider/model` (e.g., `deepseek/deepseek-chat`)

Then it loads a provider manifest and builds:

- a `ProtocolManifest` (typed manifest)
- a `Pipeline` (operator graph) from the manifest
- an `HttpTransport` configured by manifest + environment knobs

### 2.2 Chat streaming flow (recommended)

The most common flow is streaming chat:

1. Application builds messages: `Message`, `ContentBlock` (multimodal), tools (`ToolDefinition`)
2. Application uses the builder API: `client.chat().messages(...).stream().execute_stream()`
3. The client compiles a `UnifiedRequest` into provider payload via `ProtocolManifest::compile_request`
4. `HttpTransport` performs the request and returns a byte stream
5. `Pipeline` decodes bytes → frames → filters → accumulates → emits normalized `StreamingEvent`
6. Application consumes events (SSE-friendly):
   - `PartialContentDelta`
   - `PartialToolCall` / `ToolCallStarted`
   - `Metadata`
   - `StreamEnd` / `StreamError`

### 2.3 Non-streaming flow

Non-streaming is implemented by running the same pipeline and collecting events into a `UnifiedResponse`.

---

## 3) Protocol layer (`src/protocol/`)

### 3.1 Key types

- **`ProtocolManifest`**: parsed/typed representation of provider manifest YAML
- **`UnifiedRequest`**: provider-agnostic request structure used by the runtime

### 3.2 Loading (`ProtocolLoader`)

`ProtocolLoader` resolves provider manifests from:

- explicit `ProtocolLoader::with_base_path(...)`
- env (`AI_PROTOCOL_DIR` / `AI_PROTOCOL_PATH`)
- common relative paths (`ai-protocol/`, `../ai-protocol/`, …)
- GitHub raw URLs as a last resort

It is intentionally “developer-friendly” for local workflows.

### 3.3 Validation (`ProtocolValidator`)

The runtime validates manifests via JSON Schema:

- **Local schema** (developer path)
- **GitHub canonical schema** (preferred)
- **Embedded canonical schema** (offline-safe for published crates)
- **Minimal built-in schema** (last resort; allows runtime to operate with basic checks)

This strikes a balance between correctness and offline usability.

---

## 4) Pipeline layer (`src/pipeline/`): operator interpreter

The pipeline is constructed dynamically from the manifest.

Typical operator stages (conceptual):

- **Decoder**: bytes → frames (SSE / JSON lines / provider-specific variants)
- **Selector**: frame filtering by JSONPath-like predicates
- **Accumulator**: stateful assembly (e.g., tool-call arguments split across chunks)
- **FanOut**: multi-candidate expansion when the provider emits multiple candidates
- **EventMapper**: normalized `StreamingEvent` emission

Design constraints:

- streaming normalization must be stable across providers
- once any event is emitted, the runtime avoids retrying to prevent duplicate user-visible output

---

## 5) Client layer (`src/client/`): public runtime API

### 5.1 Entry point: `AiClient`

`AiClient::new("provider/model")` is the simplest runtime entry.

`AiClientBuilder` configures runtime knobs:

- fallback models: `with_fallbacks(Vec<String>)`
- strict streaming validation: `strict_streaming(bool)`
- backpressure: `max_inflight(n)`
- circuit breaker: `circuit_breaker_default()`
- rate limiting: `rate_limit_rps(...)`
- protocol path / hot reload: `protocol_path(...)`, `hot_reload(true)`

### 5.2 Chat API: builder style

Chat uses a builder (intentionally small surface area):

- `client.chat() -> ChatRequestBuilder`
- `messages(...)`, `temperature(...)`, `max_tokens(...)`
- `tools(...)`, `tool_choice(...)`
- `execute()` (non-streaming)
- `execute_stream()` (streaming)
- `execute_stream_with_cancel()` / `execute_stream_with_cancel_and_stats()`

### 5.3 Services (service discovery / management endpoints)

`EndpointExt` provides generic access to `services` defined in manifests, e.g.:

- `list_remote_models`
- `call_service("get_balance")`

No provider-specific code is needed; services are driven by manifest definitions.

---

## 6) Resilience & controls

Resilience is implemented in two parts:

### 6.1 Policy engine (`src/client/policy`)

Policy decisions are manifest-driven and use runtime signals:

- capability validation (streaming/tools/vision/…)
- retry/fallback decisions
- “pre-decision” based on breaker/rate-limiter signals

### 6.2 Controls (`src/resilience/`)

- **Rate limiter**:
  - token bucket configured via `AI_LIB_RPS` / `AI_LIB_RPM`
  - optional adaptive mode using provider headers if `rate_limit_headers` are present
- **Circuit breaker**:
  - enable via builder or env
- **Backpressure**:
  - `max_inflight` uses a semaphore to cap concurrent streams/requests

---

## 7) Types (`src/types/`)

The types module defines the “standard schema” structures the runtime operates on:

- **Messages**:
  - `Message`, `MessageRole`
  - `MessageContent` (`Text` or `Blocks`)
  - `ContentBlock` (text/image/audio/tool_use/tool_result)
- **Tools**:
  - `ToolDefinition`, `FunctionDefinition`, `ToolCall`
- **Events**:
  - `StreamingEvent` (stable, provider-agnostic streaming surface)

These types are designed to be consistent across providers.

---

## 8) Error model (layered)

The crate has **two complementary error layers**:

### 8.1 Protocol errors (`ProtocolError`)

Used while loading/validating manifests:

- `LoadError`: IO/encoding/HTTP/YAML syntax failures
- `ValidationError`: schema mismatch or structural invalidity
- `SchemaError`: schema could not be loaded/compiled
- `NotFound`: manifest not found
- `InvalidVersion`: manifest declares unsupported protocol version

### 8.2 Runtime unified errors (`Error` + `ErrorContext`)

Used by public `Result<T>`:

- `Error::Protocol(ProtocolError)`
- `Error::Pipeline(PipelineError)`
- `Error::Transport(TransportError)`
- `Error::Validation { message, context }`
- `Error::Runtime { message, context }`
- `Error::Remote { status, class, retryable, fallbackable, ... }`

`ErrorContext` adds stable, structured fields (`field_path`, `details`, `source`) to support actionable diagnostics.

---

## 9) New modules (v0.6.5)

These modules were added in v0.6.5, bringing features from the Python reference implementation.

### 9.1 Embeddings layer (`src/embeddings/`)

Provides embedding generation and vector operations:

- **`EmbeddingClient`**: Generate embeddings via OpenAI-compatible APIs
- **`EmbeddingClientBuilder`**: Configure model, API key, dimensions, batch size
- **Types**: `Embedding`, `EmbeddingRequest`, `EmbeddingResponse`, `EmbeddingUsage`, `EmbeddingModel`
- **Vector operations**:
  - Similarity: `cosine_similarity`, `dot_product`
  - Distance: `euclidean_distance`, `manhattan_distance`
  - Utilities: `normalize_vector`, `magnitude`, `average_vectors`, `weighted_average_vectors`
  - Search: `find_most_similar` with `SimilarityMetric` enum

### 9.2 Cache layer (`src/cache/`)

Response caching with pluggable backends:

- **`CacheBackend`** trait: async interface for cache implementations
- **`MemoryCache`**: In-memory LRU cache with TTL and automatic eviction
- **`NullCache`**: No-op implementation for disabling cache
- **`CacheManager`**: High-level manager with statistics tracking
- **`CacheKey`** / **`CacheKeyGenerator`**: Deterministic SHA-256 based key generation
- **`CacheStats`**: Track hits, misses, hit ratio

### 9.3 Tokens layer (`src/tokens/`)

Token counting and cost estimation:

- **`TokenCounter`** trait: Count tokens, count messages, truncate to limit
- **Implementations**:
  - `CharacterEstimator`: Simple character-based estimation (configurable ratio)
  - `AnthropicEstimator`: Claude-specific estimation with whitespace adjustment
  - `CachingCounter`: Wrapper that caches count results
- **`get_token_counter(model)`**: Factory function returning appropriate counter
- **`ModelPricing`**: Pre-configured pricing for GPT-4o, GPT-4o-mini, Claude 3.5 Sonnet, Claude 3 Haiku
- **`CostEstimate`**: Calculate and format request costs

### 9.4 Batch layer (`src/batch/`)

Request batching and batch execution:

- **`BatchConfig`**: Configure max batch size, max wait time, auto-flush
- **`BatchItem<T>`**: Wrapper with metadata (timestamp, request_id, priority)
- **`BatchCollector<T>`**: Accumulate items, detect when to flush
- **`BatchExecutor`**: Execute batches with configurable strategies
- **`BatchStrategy`**: `Parallel`, `Sequential`, or `Concurrent { max_concurrency }`
- **`BatchResult<T, E>`**: Track successes, failures, execution time, success rate

### 9.5 Plugins layer (`src/plugins/`)

Extensible plugin system:

- **`Plugin`** trait: Lifecycle hooks (`on_register`, `on_before_request`, `on_after_response`, `on_error`, `on_stream_event`)
- **`PluginPriority`**: `Highest`, `High`, `Normal`, `Low`, `Lowest`
- **`PluginContext`**: Request/response context with metadata and skip/error flags
- **`CompositePlugin`**: Combine multiple plugins
- **`PluginRegistry`**: Global registry with `register`, `unregister`, `trigger_*` methods
- **`get_plugin_registry()`**: Access global singleton

**Hook system** (`src/plugins/hooks.rs`):
- `HookType`: `BeforeRequest`, `AfterResponse`, `OnError`, `OnStreamEvent`
- `Hook` / `AsyncHook` / `FnHook`: Different hook implementations
- `HookManager`: Manage and trigger hooks

**Middleware** (`src/plugins/middleware.rs`):
- `Middleware` trait: Transform request/response
- `MiddlewareChain`: Chain multiple middleware
- `MiddlewareContext`: Context passed through chain

### 9.6 Extended Telemetry (`src/telemetry/`)

Enhanced feedback system (building on v0.2.0):

- **New feedback types**:
  - `RatingFeedback`: Star ratings (1-5) with optional comment
  - `ThumbsFeedback`: Thumbs up/down with reason
  - `TextFeedback`: Free-form text feedback
  - `CorrectionFeedback`: Track user corrections with edit distance
  - `RegenerateFeedback`: Track regeneration requests
  - `StopFeedback`: Track early stop actions
- **New sinks**:
  - `InMemoryFeedbackSink`: For testing, with `get_events()`, `get_events_by_request()`
  - `ConsoleFeedbackSink`: Debug output with configurable prefix
  - `CompositeFeedbackSink`: Fan-out to multiple sinks
- **Global management**:
  - `get_feedback_sink()`: Get current global sink
  - `set_feedback_sink(sink)`: Set global sink
  - `report_feedback(event)`: Report via global sink

---

## 10) Feature-gated optional helpers

### 10.1 `routing_mvp`

Pure logic helpers to select a model id before building a runtime client:

- `CustomModelManager` (selection strategies: performance/cost/…)
- `ModelArray` (endpoint arrays with load balancing)

Enable:

```toml
ai-lib-rust = { version = "0.1", features = ["routing_mvp"] }
```

### 10.2 `interceptors`

Application-layer hooks around calls (logging/metrics/audit):

- `Interceptor`
- `InterceptorPipeline`
- `RequestContext`

Enable:

```toml
ai-lib-rust = { version = "0.1", features = ["interceptors"] }
```

---

## 11) Recommended usage patterns (runtime-first)

- Prefer **`provider/model`** IDs and protocol manifests over any provider enums.
- Prefer the **chat builder API** (`client.chat()...`) for a small, stable public surface.
- Keep provider-specific differences in **protocol files**, not in Rust code.
- Use `EndpointExt` for service discovery and management calls declared in manifests.

---

## 12) Testing & offline behavior

The test suite is designed to run offline:

- manifests can be loaded from local `ai-protocol` checkout if present
- JSON Schema validation works offline via **embedded `schema_v1.json`**

If GitHub and local paths are unavailable, validation falls back to a minimal schema and basic checks to preserve runtime usability.

