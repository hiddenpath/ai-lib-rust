//! Integration tests for AiClient with ai-protocol-mock.
//!
//! Requires ai-protocol-mock server running. Set MOCK_HTTP_URL=http://localhost:4010
//! and run with: cargo test client_mock -- --ignored --nocapture

use ai_lib_rust::{AiClientBuilder, Message};

#[tokio::test]
#[ignore = "requires ai-protocol-mock server; run with: cargo test client_mock -- --ignored --nocapture"]
async fn test_chat_completion_with_mock() {
    let mock_url = std::env::var("MOCK_HTTP_URL").ok();
    if mock_url.is_none() {
        eprintln!("MOCK_HTTP_URL not set, skipping mock integration test");
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
        .expect("Failed to build client");

    let messages = vec![Message::user("Hello")];
    let resp = client
        .chat()
        .messages(messages)
        .execute()
        .await
        .expect("Chat request failed");

    assert!(!resp.content.is_empty());
}
