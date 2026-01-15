# Changelog

All notable changes to this project will be documented in this file.

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

