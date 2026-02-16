# Changelog

All notable changes to this project will be documented in this file.

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

