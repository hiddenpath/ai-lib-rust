//! Benchmarks for streaming pipeline performance
//!
//! This benchmark measures:
//! - SSE frame decoding speed
//! - Event mapping throughput
//! - Tool call accumulation performance
//! - Pipeline operator overhead

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};

/// Sample SSE frames (OpenAI format)
const SSE_FRAMES: &[&str] = &[
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":""},"finish_reason":null}]}"#,
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#,
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":" there"},"finish_reason":null}]}"#,
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{"content":"!"},"finish_reason":null}]}"#,
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#,
    "data: [DONE]",
];

/// Sample SSE frames with tool calls
const SSE_TOOL_CALL_FRAMES: &[&str] = &[
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{"role":"assistant","content":null,"tool_calls":[{"index":0,"id":"call_abc123","type":"function","function":{"name":"get_weather","arguments":""}}]},"finish_reason":null}]}"#,
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"lo"}}]},"finish_reason":null}]}"#,
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"cation"}}]},"finish_reason":null}]}"#,
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"\": \"To"}}]},"finish_reason":null}]}"#,
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"kyo\"}"}}]},"finish_reason":null}]}"#,
    r#"data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1694268190,"model":"gpt-4o","choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#,
    "data: [DONE]",
];

fn bench_sse_parsing(c: &mut Criterion) {
    let mut group = c.benchmark_group("sse_parsing");

    // Benchmark parsing a single SSE frame
    let frame = SSE_FRAMES[1]; // A typical content delta frame
    group.throughput(Throughput::Bytes(frame.len() as u64));

    group.bench_function("parse_single_frame", |b| {
        b.iter(|| {
            let data = black_box(frame).strip_prefix("data: ").unwrap();
            let _: serde_json::Value = serde_json::from_str(data).unwrap();
        })
    });

    // Benchmark parsing all frames in sequence
    let all_frames: String = SSE_FRAMES.join("\n\n");
    group.throughput(Throughput::Bytes(all_frames.len() as u64));

    group.bench_function("parse_all_frames", |b| {
        b.iter(|| {
            for line in black_box(&all_frames).lines() {
                if let Some(data) = line.strip_prefix("data: ") {
                    if data != "[DONE]" {
                        let _: serde_json::Value = serde_json::from_str(data).unwrap();
                    }
                }
            }
        })
    });

    group.finish();
}

fn bench_json_path_extraction(c: &mut Criterion) {
    let mut group = c.benchmark_group("json_path_extraction");

    // Parse a sample frame once
    let frame_data = SSE_FRAMES[1].strip_prefix("data: ").unwrap();
    let json: serde_json::Value = serde_json::from_str(frame_data).unwrap();

    group.bench_function("extract_content_manual", |b| {
        b.iter(|| {
            // Manual path extraction (typical implementation)
            let content = black_box(&json)
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("delta"))
                .and_then(|d| d.get("content"))
                .and_then(|c| c.as_str());
            black_box(content)
        })
    });

    group.bench_function("extract_finish_reason", |b| {
        b.iter(|| {
            let reason = black_box(&json)
                .get("choices")
                .and_then(|c| c.get(0))
                .and_then(|c| c.get("finish_reason"))
                .and_then(|r| r.as_str());
            black_box(reason)
        })
    });

    group.finish();
}

fn bench_tool_call_accumulation(c: &mut Criterion) {
    let mut group = c.benchmark_group("tool_call_accumulation");

    // Parse all tool call frames
    let frames: Vec<serde_json::Value> = SSE_TOOL_CALL_FRAMES
        .iter()
        .filter_map(|f| f.strip_prefix("data: "))
        .filter(|d| *d != "[DONE]")
        .map(|d| serde_json::from_str(d).unwrap())
        .collect();

    group.throughput(Throughput::Elements(frames.len() as u64));

    group.bench_function("accumulate_tool_calls", |b| {
        b.iter(|| {
            let mut accumulated_args = String::new();
            let mut tool_id = String::new();
            let mut tool_name = String::new();

            for frame in black_box(&frames) {
                if let Some(tool_calls) = frame
                    .get("choices")
                    .and_then(|c| c.get(0))
                    .and_then(|c| c.get("delta"))
                    .and_then(|d| d.get("tool_calls"))
                    .and_then(|tc| tc.as_array())
                {
                    for tc in tool_calls {
                        if let Some(id) = tc.get("id").and_then(|i| i.as_str()) {
                            tool_id = id.to_string();
                        }
                        if let Some(func) = tc.get("function") {
                            if let Some(name) = func.get("name").and_then(|n| n.as_str()) {
                                tool_name = name.to_string();
                            }
                            if let Some(args) = func.get("arguments").and_then(|a| a.as_str()) {
                                accumulated_args.push_str(args);
                            }
                        }
                    }
                }
            }

            black_box((tool_id, tool_name, accumulated_args))
        })
    });

    group.finish();
}

fn bench_frame_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("frame_throughput");

    // Simulate processing many frames (realistic streaming scenario)
    let repeated_frames: Vec<&str> = SSE_FRAMES
        .iter()
        .cycle()
        .take(100)
        .copied()
        .collect();

    group.throughput(Throughput::Elements(repeated_frames.len() as u64));

    group.bench_function("process_100_frames", |b| {
        b.iter(|| {
            let mut content = String::new();
            for frame in black_box(&repeated_frames) {
                if let Some(data) = frame.strip_prefix("data: ") {
                    if data != "[DONE]" {
                        if let Ok(json) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(c) = json
                                .get("choices")
                                .and_then(|c| c.get(0))
                                .and_then(|c| c.get("delta"))
                                .and_then(|d| d.get("content"))
                                .and_then(|c| c.as_str())
                            {
                                content.push_str(c);
                            }
                        }
                    }
                }
            }
            black_box(content)
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sse_parsing,
    bench_json_path_extraction,
    bench_tool_call_accumulation,
    bench_frame_throughput,
);
criterion_main!(benches);
