//! Tavily Search Tool Calling Example
//!
//! This example demonstrates how to use tool calling with the Tavily search API.
//! It shows a complete workflow:
//! 1. Define the Tavily search tool
//! 2. Send a request with the tool to a capable model
//! 3. Parse the tool calls from the model response
//! 4. Execute the Tavily search
//! 5. Send the results back to the model for final response
//!
//! Prerequisites:
//! Set one of the following environment variables:
//! - DEEPSEEK_API_KEY (recommended for tool calling)
//! - OPENAI_API_KEY
//! - ANTHROPIC_API_KEY
//! - GROQ_API_KEY
//! - TAVILY_API_KEY (for actual Tavily searches) - optional, can use mock
//!
//! Usage:
//!   # Use DeepSeek (recommended)
//!   $env:DEEPSEEK_API_KEY="your_key"; cargo run --example tavily_tool_calling
//!
//!   # Or use OpenAI
//!   $env:OPENAI_API_KEY="your_key"; cargo run --example tavily_tool_calling -- --provider openai

use ai_lib_rust::types::tool::{FunctionDefinition, ToolCall, ToolDefinition, ToolResult};
use ai_lib_rust::types::message::{ContentBlock, MessageContent, MessageRole};
use ai_lib_rust::{AiClient, Message};
use serde_json::{json, Value};
use std::env;

/// Define the Tavily search tool
fn tavily_search_tool() -> ToolDefinition {
    ToolDefinition {
        tool_type: "function".to_string(),
        function: FunctionDefinition {
            name: "tavily_search".to_string(),
            description: Some(
                "Search the web using Tavily API for real-time information".to_string(),
            ),
            parameters: Some(json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "The search query to find information"
                    },
                    "search_depth": {
                        "type": "string",
                        "enum": ["basic", "advanced"],
                        "default": "basic",
                        "description": "Search depth (basic for quick results, advanced for comprehensive)"
                    },
                    "max_results": {
                        "type": "integer",
                        "minimum": 1,
                        "maximum": 10,
                        "default": 5,
                        "description": "Maximum number of search results"
                    }
                },
                "required": ["query"]
            })),
        },
    }
}

/// Mock Tavily search implementation
/// In production, this would call the actual Tavily API
async fn mock_tavily_search(
    query: &str,
    _depth: &str,
    _max_results: i32,
) -> Result<Value, Box<dyn std::error::Error>> {
    println!("\n🔍 Executing Tavily search for: {}", query);

    // Simulated search results
    let results = json!({
        "results": [
            {
                "title": "Latest News about Rust Programming",
                "url": "https://example.com/rust-news",
                "content": "Rust 1.82 released with improved performance and new features...",
                "published_date": "2025-02-08"
            },
            {
                "title": "Rust Web Framework Benchmarks 2025",
                "url": "https://example.com/rust-benchmarks",
                "content": "Comparison of Actix, Axum, and Rocket frameworks...",
                "published_date": "2025-02-05"
            },
            {
                "title": "Rust AI Libraries Overview",
                "url": "https://example.com/rust-ai",
                "content": "A comprehensive guide to AI/ML libraries in Rust ecosystem...",
                "published_date": "2025-02-01"
            }
        ],
        "response_time": "0.423s",
        "query": query
    });

    Ok(results)
}

/// Process tool calls from model response
async fn process_tool_calls(
    tool_calls: &[ToolCall],
) -> Result<Vec<ToolResult>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();

    for tool_call in tool_calls {
        println!("\n📌 Tool Call: {}", tool_call.name);
        println!("   ID: {}", tool_call.id);
        println!("   Arguments: {}", serde_json::to_string_pretty(&tool_call.arguments)?);

        let result = if tool_call.name == "tavily_search" {
            let query = tool_call
                .arguments
                .get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let depth = tool_call
                .arguments
                .get("search_depth")
                .and_then(|v| v.as_str())
                .unwrap_or("basic");
            let max_results = tool_call
                .arguments
                .get("max_results")
                .and_then(|v| v.as_i64())
                .unwrap_or(5) as i32;

            match mock_tavily_search(query, depth, max_results).await {
                Ok(search_results) => ToolResult {
                    tool_use_id: tool_call.id.clone(),
                    content: search_results,
                    is_error: false,
                },
                Err(e) => ToolResult {
                    tool_use_id: tool_call.id.clone(),
                    content: json!({ "error": e.to_string() }),
                    is_error: true,
                },
            }
        } else {
            ToolResult {
                tool_use_id: tool_call.id.clone(),
                content: json!({ "error": format!("Unknown tool: {}", tool_call.name) }),
                is_error: true,
            }
        };

        results.push(result);
    }

    Ok(results)
}

/// Get provider from arguments or environment
fn get_provider() -> String {
    let args: Vec<String> = env::args().collect();

    // Check for --provider argument
    if let Some(idx) = args.iter().position(|arg| arg == "--provider") {
        if let Some(provider) = args.get(idx + 1) {
            return format!("{}/auto", provider);
        }
    }

    // Auto-detect from available environment variables
    if env::var("DEEPSEEK_API_KEY").is_ok() {
        "deepseek/auto".to_string()
    } else if env::var("OPENAI_API_KEY").is_ok() {
        "openai/auto".to_string()
    } else if env::var("ANTHROPIC_API_KEY").is_ok() {
        "anthropic/auto".to_string()
    } else if env::var("GROQ_API_KEY").is_ok() {
        "groq/auto".to_string()
    } else {
        eprintln!("❌ Error: No API key found in environment variables.");
        eprintln!("Please set one of:");
        eprintln!("  - DEEPSEEK_API_KEY");
        eprintln!("  - OPENAI_API_KEY");
        eprintln!("  - ANTHROPIC_API_KEY");
        eprintln!("  - GROQ_API_KEY");
        std::process::exit(1);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    println!("🚀 Tavily Search Tool Calling Example\n");

    let provider = get_provider();
    println!("📦 Using provider: {}\n", provider);

    // Create client
    let client = AiClient::new(&provider).await?;

    // First request: Ask the model to search for information
    let messages = vec![
        Message::system(
            "You are a helpful research assistant. When asked about topics, use the tavily_search tool to find current information."
        ),
        Message::user(
            "What are the latest developments in Rust programming? Please search for recent news and updates."
        ),
    ];

    // Force tool usage
    let tool_choice = json!({
        "type": "function",
        "function": { "name": "tavily_search" }
    });

    println!("📤 Sending initial request with tool definition...\n");

    let resp = client
        .chat()
        .messages(messages)
        .temperature(0.7)
        .max_tokens(1024)
        .tools(vec![tavily_search_tool()])
        .tool_choice(tool_choice)
        .execute()
        .await?;

    println!("✅ Initial response received");
    println!("   Content: {}", resp.content);

    // Check if model called tools
    if resp.tool_calls.is_empty() {
        println!("\n⚠️  No tool calls in response. Some models may not support tool calling.");
        println!("   Response: {}", resp.content);
        return Ok(());
    }

    // Process tool calls
    println!("\n🔄 Processing {} tool call(s)...", resp.tool_calls.len());
    let tool_results = process_tool_calls(&resp.tool_calls).await?;

    // Second request: Send tool results back to model for final response
    println!("\n📤 Sending tool results back to model...\n");

    let mut follow_up_messages = vec![
        Message::system("You are a helpful research assistant."),
        Message::user(
            "What are the latest developments in Rust programming? Please search for recent news and updates."
        ),
    ];

    // Add assistant's tool call as a content block
    let mut assistant_blocks = vec![ContentBlock::text(&resp.content)];
    for tool_call in &resp.tool_calls {
        assistant_blocks.push(ContentBlock::ToolUse {
            id: tool_call.id.clone(),
            name: tool_call.name.clone(),
            input: tool_call.arguments.clone(),
        });
    }
    follow_up_messages.push(Message::with_content(
        MessageRole::Assistant,
        MessageContent::blocks(assistant_blocks),
    ));

    // Add tool results
    for result in tool_results {
        follow_up_messages.push(Message::with_content(
            MessageRole::User,
            MessageContent::blocks(vec![ContentBlock::ToolResult {
                tool_use_id: result.tool_use_id,
                content: result.content,
            }]),
        ));
    }

    // Get final response
    let final_resp = client
        .chat()
        .messages(follow_up_messages)
        .temperature(0.7)
        .max_tokens(2048)
        .execute()
        .await?;

    println!("✅ Final response received\n");
    println!("📝 Assistant Response:\n{}", final_resp.content);

    if let Some(usage) = final_resp.usage {
        println!("\n📊 Token Usage:");
        if let Some(prompt_tokens) = usage.get("prompt_tokens").and_then(|v| v.as_u64()) {
            println!("   Prompt tokens: {}", prompt_tokens);
        }
        if let Some(completion_tokens) = usage.get("completion_tokens").and_then(|v| v.as_u64()) {
            println!("   Completion tokens: {}", completion_tokens);
        }
    }

    println!("\n✨ Example completed successfully!");

    Ok(())
}
