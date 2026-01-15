use ai_lib_rust::client::AiClient;
use ai_lib_rust::protocol::UnifiedRequest;
use ai_lib_rust::types::message::Message;
use ai_lib_rust::types::tool::{FunctionDefinition, ToolDefinition};
use reqwest::header::{HeaderMap, HeaderValue};

#[tokio::test]
async fn test_rate_limiter_header_extraction() {
    // Ensuring protocol directory is correctly set for manifest loading
    std::env::set_var("AI_PROTOCOL_PATH", "../../ai-protocol");

    // 1. Setup client with a real manifest (deepseek as it has simple headers)
    let client = AiClient::new("deepseek/deepseek-chat")
        .await
        .expect("Failed to create client");

    // 2. Mock a response with rate limit headers
    let mut headers = HeaderMap::new();
    headers.insert(
        "x-ratelimit-remaining-requests",
        HeaderValue::from_str("0").unwrap(),
    );
    headers.insert(
        "x-ratelimit-reset-requests",
        HeaderValue::from_str("60").unwrap(),
    ); // Reset in 60 seconds

    // 3. Update limits manually
    client.update_rate_limits(&headers).await;

    // 4. Check if rate limiter now predicts a wait
    let sig = client.signals().await;
    let wait_ms = sig.rate_limiter.unwrap().estimated_wait_ms;

    assert!(
        wait_ms.is_some(),
        "Rate limiter should predict a wait when remaining is 0"
    );
    assert!(wait_ms.unwrap() > 0);
}

#[tokio::test]
async fn test_policy_engine_validation() {
    std::env::set_var("AI_PROTOCOL_PATH", "../../ai-protocol");
    let client = AiClient::new("openai/gpt-4o-mini")
        .await
        .expect("Failed to create client");

    // Case 1: Request tools with a model that supports them
    let mut req = UnifiedRequest::default();
    req.operation = "chat".to_string();
    let tools = vec![ToolDefinition {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "test".to_string(),
            description: Some("test".to_string()),
            parameters: Some(serde_json::json!({})),
        },
    }];

    let result = client.validate_request(
        &client
            .chat()
            .tools(tools)
            .messages(vec![Message::user("hi")]),
    );
    assert!(result.is_ok(), "OpenAI should support tools");
}
