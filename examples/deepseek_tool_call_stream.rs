//! Real DeepSeek tool-call streaming example (no mock).
//!
//! This example verifies that:
//! - tools + tool_choice are sent correctly
//! - the model emits tool call deltas
//! - the runtime accumulates arguments and parses them to JSON
//!
//! Prerequisites:
//! - Set `DEEPSEEK_API_KEY`
//!
//! Run:
//!   DEEPSEEK_API_KEY=your_key cargo run --example deepseek_tool_call_stream

use ai_lib_rust::prelude::*;

fn web_search_tool() -> ai_lib_rust::types::tool::ToolDefinition {
    ai_lib_rust::types::tool::ToolDefinition {
        tool_type: "function".to_string(),
        function: ai_lib_rust::types::tool::FunctionDefinition {
            name: "web_search".to_string(),
            description: Some("Search the web for up-to-date information.".to_string()),
            parameters: Some(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query" },
                    "top_k": { "type": "integer", "minimum": 1, "maximum": 5, "default": 3 }
                },
                "required": ["query"]
            })),
        },
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Enable debug logs for tool-call delta inspection:
    // - set AI_LIB_DEBUG_TOOLCALL=1
    // - optionally set RUST_LOG=ai_lib_rust::pipeline::event_map=debug
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("Error: DEEPSEEK_API_KEY environment variable is not set.");
        eprintln!(
            "Run with: DEEPSEEK_API_KEY=your_key cargo run --example deepseek_tool_call_stream"
        );
        std::process::exit(1);
    }

    let client = Provider::DeepSeek.model("deepseek-chat").build_client().await?;

    let messages = vec![
        Message::system("You are a helpful assistant. You MUST call the provided tool before answering."),
        Message::user("Call the web_search tool EXACTLY ONCE with JSON arguments {\"query\":\"latest stable Rust version\",\"top_k\":3}. Do not answer in natural language."),
    ];

    // Force tool usage:
    // OpenAI style: {"type":"function","function":{"name":"web_search"}}
    let tool_choice = serde_json::json!({
        "type": "function",
        "function": { "name": "web_search" }
    });

    let req = ChatCompletionRequest::new(messages)
        .temperature(0.0)
        .max_tokens(256)
        .tools(vec![web_search_tool()])
        .tool_choice(tool_choice);

    let resp = client.chat_completion(req).await?;

    println!("\n\n--- Content ---\n{}", resp.content);
    println!("\n--- Tool calls ---\n{:#?}", resp.tool_calls);
    println!("\n--- Usage ---\n{:#?}", resp.usage);

    Ok(())
}
