# Runtime Enhancement Backlog (Stable Track)

This document turns design reports into **actionable, reviewable issues** with scope and acceptance criteria.
Focus: **no large refactors**, maximize correctness, observability, and protocol-driven behavior.

## Runtime (ai-lib-rust)

### P0 — Stable client request id for linkage (always present) ✅ COMPLETED
- **Why**: multi-candidate choice feedback is impossible without a stable id, and providers don't always return one.
- **Scope**: `AiClient` request execution + `CallStats`.
- **Acceptance criteria**
  - ✅ Generate `client_request_id` for every call.
  - ✅ Send it as `x-ai-protocol-request-id` header (best-effort).
  - ✅ Expose it via `CallStats.client_request_id` regardless of provider headers.
- **Status**: Implemented in `CallStats.client_request_id` and `HttpTransport`.

### P0 — Optional feedback hook (default no-op)
- **Why**: runtime must not force telemetry; apps decide collection/storage.
- **Scope**: new `telemetry` module + `AiClient::report_feedback()`.
- **Acceptance criteria**
  - Provide `FeedbackEvent::ChoiceSelection` structure with minimal fields.
  - Provide injectable `FeedbackSink` via builder; default is no-op.
  - Never blocks generation path; feedback is explicitly called by app.

### P0 — Respect `Retry-After` in retry loop (HTTP 429/503 etc.)
- **Why**: exponential backoff is good, but providers often return `Retry-After`; ignoring it causes avoidable failures and wasted quota.
- **Scope**: `AiClient::execute_request` retry loop.
- **Acceptance criteria**
  - If response includes header `retry-after: <seconds>`, the next retry delay uses that value (bounded by `max_delay_ms` when configured).
  - Keep existing exponential backoff as fallback when header is missing/invalid.
  - Log retry reason with `http_status`, `attempt`, and `retry_after_ms` (when present).

### P0 — Log stable request identifiers on remote failures
- **Why**: production debugging needs request IDs (provider support varies).
- **Scope**: error path in `AiClient::execute_request`.
- **Acceptance criteria**
  - For non-2xx responses, include `request_id` field in logs when available from headers:
    - `x-request-id`, `request-id`, `x-amzn-requestid`, `cf-ray` (best-effort)
  - No API changes required.

### P1 — Remote error context (typed, policy-friendly) **without breaking semver**
- **Why**: policy layers need structured data (status/class/retry_after/request_id/model/operation).
- **Scope**: internal representation; keep public `Error` stable.
- **Acceptance criteria**
  - Introduce an internal `RemoteErrorContext` (not part of the public error enum), used for logging and internal decisions.
  - Policy-relevant fields are logged consistently (status, class, retryable, fallbackable, request_id, endpoint, model, operation).

### P1 — Unified per-call stats (`CallStats`) emitted by the client
- **Why**: model selection needs latency/usage/error_class in a stable shape.
- **Scope**: `AiClient` public API addition (non-breaking via new method/field, not changing existing signatures).
- **Acceptance criteria**
  - Provide a way to obtain `CallStats` for each call:
    - `latency_ms`, `retry_count`, `error_class` (if failed), `usage` (if available), `endpoint`
  - Default path remains unchanged for existing callers.

### P2 — Protocol strictness toggle for streaming configuration completeness
- **Why**: missing `streaming.*_path` should be a validation failure in strict mode; auto-inference should be opt-in.
- **Scope**: `ProtocolManifest` validation.
- **Acceptance criteria**
  - Add a `strict_streaming: bool` setting (env or builder option).
  - When enabled, missing required streaming config (decoder + content/tool/usage paths when tools/usage are expected) yields `Error::Validation`.

### P2 — Decoder vs EventMapper plugin boundaries (incremental)
- **Why**: avoid “one adapter does everything”; support more providers via data-driven mapping.
- **Scope**: keep current pipeline structure; just clarify extension points.
- **Acceptance criteria**
  - Document and enforce: decoder produces JSON frames; event mapper does semantic extraction (content/tool/usage).
  - Add at least one additional decoder format or robustness improvement behind feature flag (optional).

## Manifest / AI-Protocol feedback (backfeed items)

### M0 — Treat `streaming.decoder.format` as the canonical "protocol"
- **Acceptance criteria**
  - Docs/spec explicitly state: decoder.format = protocol-level framing (SSE/NDJSON/JSON-chunk…).
  - Avoid introducing parallel `endpoint.protocol` unless it is an alias.

### M0 — Tool-call mapping sourced from `tooling.tool_use` (already in progress)
- **Acceptance criteria**
  - Providers supply `tooling.tool_use.{id_path,name_path,input_path,input_format}`.
  - Optional `index_path` supported for streaming deltas (id only appears in first chunk).

### M1 — Standardize `usage_path` for streaming
- **Acceptance criteria**
  - Schema supports `streaming.usage_path`.
  - Provider manifests fill it where applicable; runtime extracts usage via the default mapper.

