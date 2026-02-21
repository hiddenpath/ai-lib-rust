//! Integration tests for streaming responses
//!
//! Uses ai-protocol-mock when MOCK_HTTP_URL is set. Run with:
//!   MOCK_HTTP_URL=http://localhost:4010 cargo test integration::streaming -- --ignored --nocapture

use ai_lib_rust::prelude::*;
use ai_lib_rust::AiClientBuilder;
use futures::StreamExt;
use crate::integration::mock_server::MockServerFixture;

#[tokio::test]
async fn test_sse_streaming_response_mockito() {
    let fixture = MockServerFixture::new().await;

    let _mock = fixture
        .mock_sse_stream(
            "/v1/chat/completions",
            vec![
                "data: {\"choices\":[{\"delta\":{\"role\":\"assistant\"},\"index\":0}]}",
                "data: {\"choices\":[{\"delta\":{\"content\":\"Hello\"},\"index\":0}]}",
                "data: {\"choices\":[{\"delta\":{\"content\":\" World\"},\"index\":0}]}",
                "data: [DONE]",
            ],
        )
        .await;

    // In-process mockito test - no external server
}

#[tokio::test]
#[ignore = "requires ai-protocol-mock; run with: MOCK_HTTP_URL=http://localhost:4010 cargo test test_sse_streaming_via_mock -- --ignored --nocapture"]
async fn test_sse_streaming_via_mock() {
    let mock_url = std::env::var("MOCK_HTTP_URL").ok();
    if mock_url.is_none() {
        eprintln!("MOCK_HTTP_URL not set, skipping");
        return;
    }

    let manifest_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("protocols");

    let client = AiClientBuilder::new()
        .protocol_path(manifest_path.to_string_lossy().to_string())
        .base_url_override(mock_url.as_ref().unwrap().as_str())
        .build("openai/gpt-4o")
        .await
        .expect("build client");

    let mut stream = client
        .chat()
        .messages(vec![Message::user("Hi")])
        .stream()
        .execute_stream()
        .await
        .expect("start stream");

    let mut collected = String::new();
    while let Some(ev) = stream.next().await {
        if let Ok(StreamingEvent::PartialContentDelta { content }) = ev {
            collected.push_str(&content);
        }
    }
    assert!(!collected.is_empty(), "expected non-empty streamed content");
}

#[tokio::test]
async fn test_streaming_cancellation() {
    // Test that cancelling a stream properly cleans up resources
    // This would require a mock server that can detect connection drops
}

#[tokio::test]
async fn test_ndjson_streaming() {
    // Test NDJSON/JSONL streaming format
    let fixture = MockServerFixture::new().await;
    
    let body = r#"{"content":"Chunk 1"}
{"content":"Chunk 2"}
{"content":"Chunk 3"}
"#;
    
    let _mock = fixture
        .mock_json_response("/v1/chat/completions", 200, body)
        .await;
    
    // Test NDJSON parsing
}
