# Changelog

All notable changes to this project will be documented in this file.

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

