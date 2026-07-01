# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

## 1.0.0 - 2026-07-01

### Milestone

- **Wave-5 v1.0.0**: E/P separation frozen (`ai-lib-core` / `ai-lib-contact` / facade); PT-073g sign-off; pins `@ailib-official/ai-protocol@1.0.0`.

### Added

- PT-073g remediation: transport panic fix, compliance prod runners, CI protocol pin, fail-closed fixtures (#12–#17).

### Changed

- Workspace crate versions aligned to **1.0.0** (`ai-lib-core` → `ai-lib-contact` → `ai-lib-rust`).
- README feature matrix and version examples updated for 1.0.0.

## 0.9.6 - 2026-05-07

### Added

- **Credential chain (PT-074)**: `ai-lib-core` now resolves protocol credentials with explicit builder override, manifest-declared env vars, conventional provider env fallback, and (when the `keyring` feature is enabled) native OS keyring fallback.
- **Credential resolver tests (PT-074-B-FIX)**: 5 new `credentials::tests` cases covering the missing-credential diagnostic path and the V1/V2 dual-`auth` divergence helper, plus 3 `transport::http::tests` cases covering `apply_auth` behavior with no secret, `query_param` attachment, and unknown-`auth_type` Bearer fallback. A `#[ignore]`-marked keyring path test documents how to validate manually with the `keyring` feature.

### Changed

- **HTTP transport credentials**: request auth now follows manifest auth metadata for bearer, custom-header/API-key, and query-param attachment, and debug logging no longer emits secret length, edge characters, or raw bytes.
- **`keyring` is now an opt-in feature (PT-074-B-FIX-1)**: the `keyring` crate moved behind a `keyring` Cargo feature in both `ai-lib-core` and `ai-lib-rust`. The feature is included in `default` so desktop usage is unchanged, but slim/container/CI builds can now disable it with `default-features = false`, removing the transitive D-Bus / libsecret / Security Framework dependency. `full` continues to enable `keyring`.
- **Credential auth is single-source (PT-074-B-FIX-2)**: `credentials::required_envs()` now scans only the active `primary_auth()` block (V2 `endpoint.auth` wins, V1 top-level `auth` is the fallback). This prevents a "V1 token + V2 attachment shape" Frankenstein resolution when both blocks are declared with divergent envs.
- **Conventional env var is canonical (PT-074-B-FIX-2)**: `credentials::conventional_envs()` returns a single `${PROVIDER_ID_UPPER_WITH_UNDERSCORES}_API_KEY` entry. Non-conventional aliases must be declared via `auth.token_env` / `auth.key_env` in the manifest.
- **`AuthConfig` field name aligned with V2 schema (PT-074-B-FIX-4)**: the custom auth header field now serializes as `header` (V2 canonical name) and accepts `header_name` only as a V1 compatibility deserialize alias. The Rust struct identifier `header_name` is unchanged.
- **Workspace crate versions** aligned to `0.9.6` for a coherent crates.io publish train (`ai-lib-core` → `ai-lib-contact` → `ai-lib-rust`).

### Diagnostics

- **Dual-`auth` drift warning (PT-074-B-FIX-3)**: `HttpTransport` emits a single `tracing::warn!` at construction when a manifest declares both `endpoint.auth` and top-level `auth` with divergent `(type, token_env, key_env)`, naming both blocks so operators can fix the manifest.
- **Unknown `auth_type` warning (PT-074-B-FIX-3)**: when `apply_auth` encounters an unrecognized `auth.type`, it still falls back to `Bearer Authorization` (reversibility), but emits a `tracing::warn!` once per process per offending value via `OnceLock<Mutex<HashSet<String>>>` dedup.

## 0.9.4 - 2026-04-11

### Added

- E/P boundary types: `ExecutionResult`, `ExecutionMetadata`, `ExecutionUsage` (`types::execution_result`), plus JSON serde for `StandardErrorCode` via canonical `E1xxx` strings.
- **`ai-lib-wasm` crate (PT-072)**: WASI `wasm32-wasip1` thin exports over `ai-lib-core`. Six host-facing C functions: `ailib_load_manifest`, `ailib_check_capability`, `ailib_build_chat_request`, `ailib_parse_chat_response`, `ailib_classify_error`, `ailib_extract_usage`, plus `ailib_out_*` / `ailib_err_*` memory accessors. Release binary **1.24 MB** (< 2 MB gate). Zero P-module dependencies.
- `wasm_manifest::load_manifest_validated()` in `ai-lib-core::protocol` — in-memory YAML parse + `ProtocolValidator` (works on both native and WASM).
- `WasmChatRequest` DTO in `ai-lib-wasm` for safe deserialization without requiring `Deserialize` on `UnifiedRequest`.
- **PT-073 (Rust):** full compliance YAML suite on `ai-lib-core` via `--test compliance_from_core` (shared runner with the facade `compliance` test). Wasmtime in-process harness: `cargo test -p ai-lib-wasmtime-harness --test wasm_compliance` after building `ai-lib-wasm` for `wasm32-wasip1` release.
- **GitHub Actions:** `.github/workflows/pt073-rust-core-wasm.yml` runs EP-boundary script, `cargo test -p ai-lib-core`, WASI release build, wasmtime compliance test, and `ai-lib-contact` compile smoke (checkout `ailib-official/ai-protocol`).

### Changed

- **OpenAI-compatible non-stream parsing**: fallback to standard `choices[0].message.content` / `usage` paths when V2 manifests omit V1-style `response_paths`, restoring correct DeepSeek non-stream output extraction.
- **OpenAI-compatible streaming**: prefer path-based mapping for `openai_chat` SSE decoders and honor `stream` parameter mappings from protocol manifests, restoring DeepSeek/Groq incremental `delta` delivery.
- **HTTP transport routing**: try direct plus configured local proxy routes (`AI_PROXY_URL`, `HTTPS_PROXY`, `HTTP_PROXY`), remember the last successful path, and auto-fail over on connection or region/proxy-style statuses (`403`, `407`, `451`, `502`, `503`, `504`).
- **`HttpTransport::execute_stream_response`**: added `accept_event_stream`; non-streaming calls use `Accept: application/json` instead of always requesting SSE (avoids empty `message.content` on some OpenAI-compatible providers when `stream: false`).
- **Non-stream response parsing**: if the primary `content` path is empty, try manifest `reasoning_content` / `reasoning` paths (e.g. DeepSeek reasoner-style payloads).
- **Workspace layout (PT-068)**: split into `ai-lib-core` (execution), `ai-lib-contact` (policy), `ai-lib-wasm` (WASI exports), and `ai-lib-rust` (facade re-exports, tests, examples, bins).
- **`AiClient`**: no longer wires circuit breaker / rate limiter; use `ai_lib_contact::resilience` (or the facade `ai_lib_rust::resilience`) beside the client. `SignalsSnapshot` / preflight policy focus on inflight saturation only; header-driven rate-limit updates and breaker record hooks were removed from the client path.
- `ai-lib-core`: on `wasm32` targets, `client`, `transport`, `pipeline`, `feedback`, `registry`, and optional feature modules (`embeddings`, `mcp`, etc.) are excluded via `cfg(not(target_arch = "wasm32"))`. `ProtocolValidator` on wasm uses `validate_basic` only (no `jsonschema` crate).
- `UsageInfo` now derives `serde::Serialize` (needed for WASM JSON output).
- **README rewritten**: Updated to reflect E/P separation architecture (v0.9).
- **CI**: Uses `ailib-official/ai-protocol` as protocol source.

### Fixed

- **Protocol v1→v2 consistency**: Fixed critical bug ensuring consistent manifest loading across v1 and v2 formats.
- **Clippy compliance**: Implemented `FromStr` trait for `Modality` and `AudioFormat` (RUST-001).
- **Clippy derivable_impls**: Used `#[derive(Default)]` for `FilterAction`.
- Removed missing example declaration from Cargo.toml.

## 0.9.1 - 2026-03-08

### Added

- Compliance execution expanded in `tests/compliance.rs` to cover:
  - retry decision (`res-*`)
  - message building (`msg-*`)
  - stream decoding/event mapping/tool accumulation (`str-*`)
  - request parameter mapping (`req-*`)
- Structured endpoint path compatibility test for V2 manifests (`endpoint.chat` as object with `path`).

### Changed

- V2 manifest endpoint path fields now accept both string and structured `{ path: ... }` forms via `EndpointPath`.
- Compliance suite runs without dead_code warnings in test structures.

## 0.9.0 - 2026-03-07

### Added

- Cross-repo generative manifest consumption regression test: `tests/generative_manifest_consumption.rs` validates loading and capability usage against latest `ai-protocol/v2/providers/*.yaml` (Google, DeepSeek, Qwen, Doubao).
- V2 manifest type support for `multimodal.output.video` via `VideoOutputConfig`.

### Changed

- Multimodal capability extraction now respects `output.video` declarations and propagates video output support where declared.

## 0.8.6 - 2026-02-28

### Added

- **`MessageRole::Tool`**: New variant for tool result messages in multi-turn tool calling
- **`Message::tool(tool_call_id, content)`**: Constructor for tool result messages (OpenAI API: role "tool")
- **`Message.tool_call_id`**: Optional field, required when role is Tool for OpenAI serialization
- **Driver support**: OpenAI, Anthropic, and Gemini drivers now serialize `MessageRole::Tool` to provider-native format
- **Benchmark scaffolding**: Portable `benchmarks/` scripts and config for local/autocannon validation
- **Operational docs**: Added security/secret-management guidance and helper script for benchmark secrets workflows

### Changed

- `Message` struct: added `tool_call_id: Option<String>` with `#[serde(default)]` for backward compatibility
- Runtime validator now accepts V2 manifests in mock/testing paths by skipping strict V1 schema enforcement for protocol `2.x`
- Mock fixture manifest for OpenAI now includes streaming/response mappings required by current pipeline validation

## 0.8.5 - 2026-02-20

### Added (ZeroClaw upstream response)

- **`ChatRequestBuilder::model()`**: Override model per request for single-client multi-model usage
- **`ChatRequestBuilder::tools_json()`**: Set tools from raw `Vec<serde_json::Value>` for JSON Schema integration
- **`Error::is_retryable()`**, **`Error::retry_after()`**, **`Error::error_code()`**: Convenience methods for error handling
- **`AiClient::metrics()`**: Returns `ClientMetrics` snapshot (total_requests, successful_requests, total_tokens)
- **`ClientMetrics`**: New type for cumulative client metrics

### Changed

- **Documentation**: `execute_stream_with_cancel` and `CancelHandle` Rustdoc with examples
- **Documentation**: `Arc<AiClient>` sharing pattern in README and `AiClientBuilder` (replaces Clone for ToS compliance)
- **Fallback behavior**: Model override from `ChatRequestBuilder::model()` preserved for primary client; fallback clients use their own model_id

## 0.8.0 - 2026-02-16

### Added

#### V2 Protocol Runtime Support (`protocol/v2/`)
- V2 three-ring manifest parser: Ring1 (core skeleton), Ring2 (capability mapping), Ring3 (advanced extensions)
- `ManifestV2` typed model with auto-promotion from V1 manifests
- `CapabilitiesV2` structured capability declaration (required/optional)
- Protocol version "2.0" added to supported versions for validation

#### Provider Drivers (`drivers/`)
- `ProviderDriver` trait abstraction for provider-specific API handling
- OpenAI-compatible driver (covers OpenAI, DeepSeek, Moonshot, Qwen, Groq, etc.)
- Anthropic Messages API driver with beta header support
- Gemini GenerateContent API driver
- Auto-detection of API style from provider manifest

#### MCP Tool Bridge (`mcp/`, feature: `mcp`)
- `McpToolBridge`: Namespace-based tool conversion between MCP and AI-Protocol formats
- `mcp_tools_to_protocol()`: Convert MCP tool definitions to protocol-compatible function tools
- `protocol_call_to_mcp()`: Route protocol tool calls back to MCP invocations
- `mcp_result_to_protocol()`: Convert MCP results to protocol format
- Server filtering and namespace collision detection

#### Computer Use Abstraction (`computer_use/`, feature: `computer_use`)
- `ComputerAction` enum: screenshot, click, type, scroll, browser_navigate, key_press
- `SafetyPolicy`: Configurable safety enforcement with domain allowlists and action limits
- `extract_provider_config()`: Parse provider-specific CU configuration from manifests
- Implementation style detection (screen-based vs tool-based)

#### Extended Multimodal (`multimodal/`, feature: `multimodal`)
- `MultimodalCapabilities`: Input/output modality declarations with format validation
- Vision, audio, video modality support with format and size validation
- `validate_content_modalities()`: Check content blocks against provider capabilities

#### Capability Registry (`registry/`)
- `CapabilityRegistry`: Dynamic capability detection from V2 manifests
- Status reporting for active, optional, and unavailable capabilities

#### CLI Tool (`bin/ai_protocol_cli`)
- `validate`: Validate all provider manifests (V1 + V2) and JSON schemas
- `info <provider>`: Show provider capabilities, MCP, Computer Use, multimodal details
- `list`: List all available providers with version info
- `check-compat <manifest>`: Check runtime feature compatibility requirements
- Cross-platform path resolution (removed hardcoded Windows paths)

### Changed
- V2 protocol version "2.0" now accepted in manifest validation
- README updated with V2 features, new feature flags, and version 0.8.0 references

## 0.7.1 - 2026-02-15

### Fixed
- Fix missing `response_format` field in `benches/request_compilation.rs` struct literals causing `cargo check --benches` failure
- Note: crates.io v0.7.0 package was unaffected (bench files are excluded from published crate)

## 0.7.0 - 2026-02-15

### Added

#### V2 Standard Error Codes (`error_code.rs`)
- 13-variant `StandardErrorCode` enum aligned with AI-Protocol V2 specification (E1001–E9999)
- `from_provider_code()`: Maps provider-specific error codes (e.g., `overloaded_error`, `context_length_exceeded`) to standard codes
- `from_error_class()`: Maps error class names to standard codes
- `from_http_status()`: Maps HTTP status codes to standard codes (including Anthropic 529)
- Each code carries `retryable()`, `fallbackable()`, and `category()` metadata

#### Feature Flags (Cargo features)
- 7 capability features: `embeddings`, `batch`, `guardrails`, `tokens`, `telemetry`, `routing_mvp`, `interceptors`
- `full` meta-feature for convenience
- Core feedback types (`FeedbackSink`, `FeedbackEvent`) always compiled; telemetry sinks feature-gated

#### Structured Output (`structured/`)
- JSON mode with schema validation
- Response format constraint support

#### Compliance Testing
- Cross-runtime compliance test runner (`tests/compliance.rs`)
- 20/20 test cases passing against `ai-protocol/tests/compliance/cases/`

### Changed

#### Error Classification Improvements
- All error paths now attach V2 `StandardErrorCode` to `ErrorContext` (eliminates runtime re-derivation)
- Provider error codes extracted once per error response (removed duplicate `error_code_from_body()` calls)
- Structured logging now includes `standard_code` field for observability
- `format_context()` optimized: single-buffer write instead of Vec<String> allocation

#### Documentation
- Added Chinese one-line description to all module doc headers (11 modules)
- All remaining documentation in English per project convention

#### .gitignore
- Added Chinese-named internal document patterns
- Strengthened wildcard patterns for work documents

### Fixed
- Duplicate doc comment on `call_model_with_stats()` method
- Inconsistent `should_fallback` computation in non-streaming error path

## 0.6.6 - 2026-02-06

### Added

#### Guardrails Module (`guardrails/`)
- `Guardrails` controller for content filtering and safety checks
- `GuardrailsConfig` with builder pattern for flexible configuration
- `KeywordFilter` and `PatternFilter` for rule-based content filtering
- `PiiDetector` for detecting personally identifiable information (email, phone, SSN, credit card)
- `CheckResult` and `Violation` types for structured violation reporting
- Preset configurations: `Guardrails::permissive()` and `Guardrails::strict()`

#### Benchmarks
- Added Criterion benchmark framework with three benchmark suites:
  - `protocol_loading`: Protocol manifest loading performance
  - `request_compilation`: Request building and compilation benchmarks
  - `streaming_pipeline`: Streaming response processing benchmarks

#### Production Examples
- `guardrails_usage.rs`: Content filtering and PII detection demo
- `resilience_patterns.rs`: Circuit breaker and rate limiter usage
- `batch_processing.rs`: Batch collection and execution patterns
- `embeddings_similarity.rs`: Vector operations and semantic search

#### API Enhancements
- `CircuitBreakerConfig`: Added `new()`, `with_failure_threshold()`, `with_cooldown()`, `with_reset_timeout()` builder methods
- `CircuitBreaker`: Added `allow_request()`, `record_success()`, `record_failure()` convenience methods
- `RateLimiterConfig`: Added `new()`, `with_max_tokens()`, `with_refill_rate()` builder methods
- `RateLimiter`: Added `try_acquire()` for non-blocking token acquisition

### Changed

#### Code Refactoring
- Split `protocol/mod.rs` (635 lines) into focused sub-modules:
  - `protocol/error.rs`: Protocol error types
  - `protocol/request.rs`: Unified request format
  - `protocol/config.rs`: Configuration structures
  - `protocol/manifest.rs`: Protocol manifest structure

#### Documentation
- Enhanced Rustdoc documentation for 12 core modules with:
  - Professional English documentation
  - Chinese one-line introduction at module level
  - Comprehensive examples and usage guides
  - Module capability tables

#### Testing
- Added 75+ unit tests across modules:
  - `batch/collector.rs`: BatchCollector, BatchConfig, BatchItem tests
  - `embeddings/vectors.rs`: Vector operations (cosine similarity, distance metrics)
  - `resilience/circuit_breaker.rs`: Circuit breaker state machine tests
  - `resilience/rate_limiter.rs`: Token bucket algorithm tests

### Dependencies
- Added `criterion` (0.5) for benchmarking
- Added `regex` (1.10) for pattern matching in guardrails

## 0.6.5 - 2026-01-27

### Added (Features from ai-lib-python)

This release adds features learned from the Python reference implementation.

#### Embedding Support (`embeddings/`)
- `EmbeddingClient`, `EmbeddingClientBuilder` for generating embeddings
- `Embedding`, `EmbeddingRequest`, `EmbeddingResponse`, `EmbeddingUsage` types
- Vector operations: `cosine_similarity`, `dot_product`, `euclidean_distance`, `manhattan_distance`
- `normalize_vector`, `average_vectors`, `weighted_average_vectors`, `find_most_similar`

#### Response Caching (`cache/`)
- `CacheBackend` trait with `MemoryCache` and `NullCache` implementations
- `CacheManager` with TTL support and statistics
- `CacheKey`, `CacheKeyGenerator` for deterministic cache keys

#### Token Counting (`tokens/`)
- `TokenCounter` trait with `CharacterEstimator`, `AnthropicEstimator`, `CachingCounter`
- `ModelPricing` with pre-configured pricing for GPT-4o, Claude models
- `CostEstimate` for cost calculation

#### Extended Feedback System (`telemetry/`)
- New feedback types: `RatingFeedback`, `ThumbsFeedback`, `TextFeedback`, `CorrectionFeedback`, `RegenerateFeedback`, `StopFeedback`
- New sinks: `InMemoryFeedbackSink`, `ConsoleFeedbackSink`, `CompositeFeedbackSink`
- Global sink management: `get_feedback_sink()`, `set_feedback_sink()`, `report_feedback()`

#### Request Batching (`batch/`)
- `BatchConfig`, `BatchCollector` for request accumulation
- `BatchExecutor`, `BatchResult` for batch execution with configurable strategies

#### Plugin System (`plugins/`)
- `Plugin` trait with lifecycle hooks
- `PluginContext`, `PluginPriority`, `CompositePlugin`
- `PluginRegistry` for centralized management
- Hook system: `HookType`, `Hook`, `HookManager`
- Middleware: `Middleware`, `MiddlewareChain`, `MiddlewareContext`

### Dependencies
- Added `sha2` for cache key hashing
- Added `once_cell` for lazy static initialization

## 0.6.0 - 2026-01-27

### Added
- **Dist JSON 快路径（零额外解析成本）**：`ProtocolLoader` 优先从 `dist/v1/providers/*.json` 加载 provider manifest（本地与远程 URL 均支持），在不改变对外 API 的前提下提升加载速度与稳定性。
- **JSON model registry 支持**：模型注册表加载支持 `dist/v1/models/*.json` 与 `v1/models/*.yaml|yml` 混合存在的场景。

### Changed
- **加载顺序更稳健**：provider manifest 搜索顺序调整为 `dist JSON → source YAML → GitHub raw（JSON→YAML）`，保持向后兼容并减少“找不到文件”的误判。
- **更清晰的错误分类**：YAML 解析失败时区分“语法/编码问题”和“结构不匹配（缺字段/类型不符）”，便于定位问题来源。

## 0.5.1 - 2026-01-20

### Fixed
- **`mismatched_lifetime_syntaxes`**：在 `AiClient::chat()` 与 `validate_request()` 中为 `ChatRequestBuilder` 显式标注生命周期 `ChatRequestBuilder<'_>`，消除隐藏生命周期的告警。
- **`async_fn_in_trait`**：将 `EndpointExt` 的 `call_service`、`list_remote_models` 从 `async fn` 改为 `fn ... -> impl Future<Output = T> + Send`，显式给出 `Send` 约束，便于跨线程使用并消除告警。

### Changed
- **MSRV**：`rust-version` 从 1.70 提升至 1.75（`EndpointExt` 的 RPITIT 需要 1.75）。

## 0.2.0 (2026-01-04)

### Added
- `CallStats` and `AiClient::call_model_with_stats()` for per-call observability.
- Optional telemetry feedback hook (`telemetry` module):
  - `FeedbackEvent`, `ChoiceSelectionFeedback`
  - `FeedbackSink` (default: no-op)
  - `AiClient::report_feedback()`
- `client_request_id` generated per call and sent as `x-ai-protocol-request-id` header (best-effort).
- Strict streaming validation toggle:
  - `AiClientBuilder::strict_streaming(true)`
  - env `AI_LIB_STRICT_STREAMING=1`

### Changed
- Retry logic now respects `Retry-After` when present (seconds → ms, bounded by `max_delay_ms`).
- Added best-effort logging of upstream request identifiers (`x-request-id`, `request-id`, `x-amzn-requestid`, `cf-ray`).
- `PathMapper::get_path()` supports dot-index segments (e.g. `choices.0.delta.content`) in addition to bracket indexing.

### Breaking
- `HttpTransport::execute_stream_response` signature changed (adds `client_request_id`).
- `CallStats` fields changed to include `client_request_id` and `upstream_request_id`.

