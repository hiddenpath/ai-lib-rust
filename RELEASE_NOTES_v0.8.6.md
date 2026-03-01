# ai-lib-rust v0.8.6 Release Notes

**Release Date**: 2026-02-28

## Summary

This release finalizes tool-role interoperability updates, improves V2 manifest acceptance in mock/testing flows, and normalizes benchmark scaffolding for portable local usage.

## What's New

### Tool role support completion

- Added `MessageRole::Tool` and `Message::tool(tool_call_id, content)` for multi-turn tool result history
- Added `tool_call_id` field on `Message` with backward-compatible defaulting
- Updated provider drivers (OpenAI/Anthropic/Gemini) to serialize tool role messages correctly

### V2/mock flow robustness

- Runtime manifest validator now allows protocol `2.x` manifests in runtime validation paths
- Updated OpenAI mock fixture mappings (`response_paths` + `streaming`) to satisfy current pipeline requirements

### Benchmark and ops hygiene

- Added portable benchmark scaffolding under `benchmarks/` (repo-local output path)
- Added security/secret-management operational docs and helper script for benchmark workflows

## Validation

- `cargo test --test client_mock test_chat_completion_with_mock -- --ignored --nocapture` passed against local mock

