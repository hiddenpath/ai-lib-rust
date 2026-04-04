use ai_lib_rust::client::AiClient;
use ai_lib_rust::resilience::rate_limiter::{RateLimiter, RateLimiterConfig};
use ai_lib_rust::types::message::Message;
use ai_lib_rust::types::tool::{FunctionDefinition, ToolDefinition};

#[tokio::test]
async fn test_rate_limiter_header_driven_budget_predicts_wait() {
    let cfg = RateLimiterConfig::from_rps(10.0).expect("valid rps");
    let rl = RateLimiter::new(cfg);
    rl.update_budget(Some(0), Some(std::time::Duration::from_secs(60)))
        .await;
    let sig = rl.snapshot().await;
    assert!(
        sig.estimated_wait_ms.is_some(),
        "limiter should predict a wait when remaining is 0"
    );
    assert!(sig.estimated_wait_ms.unwrap() > 0);
}

#[tokio::test]
async fn test_policy_engine_validation() {
    std::env::set_var("AI_PROTOCOL_PATH", "../../ai-protocol");
    let client = AiClient::new("openai/gpt-4o-mini")
        .await
        .expect("Failed to create client");

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
