# ai-lib-rust benchmark scaffolding

This directory contains portable benchmark helpers for HTTP load testing.

## Quick start

1. Install `autocannon`: `npm install -g autocannon`
2. Edit `benchmark_config.json` for your target endpoint and payload
3. Run from this directory:
   - `pwsh ./run_autocannon.ps1`

## Output

Benchmark runs are appended to `../outputs/benchmark_results.json`.

## Notes

- API keys are read from `AI_API_KEY`, `OPENAI_API_KEY`, `API_KEY`, or `OPENAI_KEY`
- Keep this script for optional performance testing; it is not part of default CI