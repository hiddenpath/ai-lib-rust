//! MCP bridge integration tests against ai-protocol-mock.
//!
//! Requires ai-protocol-mock server running. Set MOCK_MCP_URL=http://localhost:4010/mcp
//! Run with: cargo test mcp_bridge -- --ignored --nocapture

use reqwest::Client;

const DEFAULT_MCP_URL: &str = "http://localhost:4010/mcp";

#[tokio::test]
#[ignore = "requires ai-protocol-mock server; run with: cargo test mcp_bridge -- --ignored --nocapture"]
async fn test_mcp_tools_list() {
    let mcp_url = std::env::var("MOCK_MCP_URL").unwrap_or_else(|_| DEFAULT_MCP_URL.to_string());

    let client = Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let resp = client
        .post(&mcp_url)
        .json(&body)
        .send()
        .await
        .expect("Request failed");

    assert!(resp.status().is_success());
    let data: serde_json::Value = resp.json().await.expect("Parse JSON failed");
    let result = data.get("result").expect("No result");
    let tools = result.get("tools").expect("No tools");
    assert!(tools.as_array().unwrap().len() > 0);
}

#[tokio::test]
#[ignore = "requires ai-protocol-mock server"]
async fn test_mcp_tools_call() {
    let mcp_url = std::env::var("MOCK_MCP_URL").unwrap_or_else(|_| DEFAULT_MCP_URL.to_string());

    let client = Client::new();
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {"name": "read_file", "arguments": {"path": "/tmp/test.txt"}}
    });

    let resp = client
        .post(&mcp_url)
        .json(&body)
        .send()
        .await
        .expect("Request failed");

    assert!(resp.status().is_success());
    let data: serde_json::Value = resp.json().await.expect("Parse JSON failed");
    let result = data.get("result").expect("No result");
    assert!(result.get("content").is_some());
}
