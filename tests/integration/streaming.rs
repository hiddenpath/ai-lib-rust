//! Integration tests for streaming responses

use ai_lib_rust::prelude::*;
use ai_lib_rust::AiClientBuilder;
use futures::StreamExt;
use crate::integration::mock_server::MockServerFixture;

#[tokio::test]
async fn test_sse_streaming_response() {
    let fixture = MockServerFixture::new().await;
    
    // Mock OpenAI-style SSE response
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

    // Note: This test requires modifying the client to use the mock server URL
    // For now, this is a structure for future implementation
    // We'd need to inject the base_url into the client or use a test transport
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
