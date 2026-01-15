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

    // Create a test client with base_url override
    // Note: This requires a valid model identifier like "openai/gpt-4"
    // For now, we'll skip the actual request to avoid protocol loading issues
    // let client = fixture.create_test_client("openai/gpt-4").await.unwrap();
    // TODO: Implement actual streaming test once protocol manifests are available
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
