//! Benchmarks for protocol loading performance
//!
//! This benchmark measures:
//! - YAML parsing speed
//! - JSON parsing speed (dist/ fast path)
//! - Protocol validation overhead
//! - Hot reload performance

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};

/// Sample protocol YAML for benchmarking
const SAMPLE_PROTOCOL_YAML: &str = r#"
$schema: "https://raw.githubusercontent.com/hiddenpath/ai-protocol/main/schemas/v1.json"
id: benchmark-provider
protocol_version: "1.5"
name: "Benchmark Provider"
status: stable
category: ai_provider
official_url: "https://example.com"
support_contact: "support@example.com"

endpoint:
  base_url: "https://api.example.com"
  timeout_ms: 30000

availability:
  required: true
  regions: [global]
  check:
    method: HEAD
    path: /health
    expected_status: [200]

capabilities:
  streaming: true
  tools: true
  vision: true

auth:
  type: bearer
  token_env: EXAMPLE_API_KEY

parameter_mappings:
  model: model
  messages: messages
  temperature: temperature
  max_tokens: max_tokens
  stream: stream
  tools: tools

streaming:
  decoder:
    format: sse
    strategy: openai_sse
  event_map:
    - match: "$.choices[0].delta.content != null"
      emit: PartialContentDelta
      fields:
        content: "$.choices[0].delta.content"
"#;

/// Sample protocol JSON (simulating dist/ fast path)
const SAMPLE_PROTOCOL_JSON: &str = r#"{
  "$schema": "https://raw.githubusercontent.com/hiddenpath/ai-protocol/main/schemas/v1.json",
  "id": "benchmark-provider",
  "protocol_version": "1.5",
  "name": "Benchmark Provider",
  "status": "stable",
  "category": "ai_provider",
  "official_url": "https://example.com",
  "support_contact": "support@example.com",
  "endpoint": {
    "base_url": "https://api.example.com",
    "timeout_ms": 30000
  },
  "availability": {
    "required": true,
    "regions": ["global"],
    "check": {
      "method": "HEAD",
      "path": "/health",
      "expected_status": [200]
    }
  },
  "capabilities": {
    "streaming": true,
    "tools": true,
    "vision": true
  },
  "auth": {
    "type": "bearer",
    "token_env": "EXAMPLE_API_KEY"
  },
  "parameter_mappings": {
    "model": "model",
    "messages": "messages",
    "temperature": "temperature",
    "max_tokens": "max_tokens",
    "stream": "stream",
    "tools": "tools"
  },
  "streaming": {
    "decoder": {
      "format": "sse",
      "strategy": "openai_sse"
    },
    "event_map": [
      {
        "match": "$.choices[0].delta.content != null",
        "emit": "PartialContentDelta",
        "fields": {
          "content": "$.choices[0].delta.content"
        }
      }
    ]
  }
}"#;

fn bench_yaml_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("protocol_parsing");
    group.throughput(Throughput::Bytes(SAMPLE_PROTOCOL_YAML.len() as u64));

    group.bench_function("yaml_parse", |b| {
        b.iter(|| {
            let manifest: ai_lib_rust::protocol::ProtocolManifest =
                serde_yaml::from_str(black_box(SAMPLE_PROTOCOL_YAML)).unwrap();
            black_box(manifest)
        })
    });

    group.throughput(Throughput::Bytes(SAMPLE_PROTOCOL_JSON.len() as u64));

    group.bench_function("json_parse", |b| {
        b.iter(|| {
            let manifest: ai_lib_rust::protocol::ProtocolManifest =
                serde_json::from_str(black_box(SAMPLE_PROTOCOL_JSON)).unwrap();
            black_box(manifest)
        })
    });

    group.finish();
}

fn bench_parsing_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("yaml_vs_json");

    // Compare YAML vs JSON parsing speed
    group.bench_with_input(
        BenchmarkId::new("format", "yaml"),
        &SAMPLE_PROTOCOL_YAML,
        |b, yaml| {
            b.iter(|| {
                let _: ai_lib_rust::protocol::ProtocolManifest =
                    serde_yaml::from_str(black_box(yaml)).unwrap();
            })
        },
    );

    group.bench_with_input(
        BenchmarkId::new("format", "json"),
        &SAMPLE_PROTOCOL_JSON,
        |b, json| {
            b.iter(|| {
                let _: ai_lib_rust::protocol::ProtocolManifest =
                    serde_json::from_str(black_box(json)).unwrap();
            })
        },
    );

    group.finish();
}

fn bench_manifest_operations(c: &mut Criterion) {
    let manifest: ai_lib_rust::protocol::ProtocolManifest =
        serde_json::from_str(SAMPLE_PROTOCOL_JSON).unwrap();

    let mut group = c.benchmark_group("manifest_operations");

    group.bench_function("supports_capability_check", |b| {
        b.iter(|| {
            let _ = black_box(&manifest).supports_capability("streaming");
            let _ = black_box(&manifest).supports_capability("tools");
            let _ = black_box(&manifest).supports_capability("vision");
            let _ = black_box(&manifest).supports_capability("multimodal");
        })
    });

    group.bench_function("get_base_url", |b| {
        b.iter(|| black_box(&manifest).get_base_url())
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_yaml_parsing,
    bench_parsing_comparison,
    bench_manifest_operations,
);
criterion_main!(benches);
