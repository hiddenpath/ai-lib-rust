//! Basic usage example (developer-friendly facade)
//!
//! This example demonstrates how to use the unified client interface
//! to interact with AI models through the protocol runtime.
//!
//! API Keys are configured via environment variables:
//! - DEEPSEEK_API_KEY for Deepseek
//!
//! Usage (PowerShell):
//!   $env:DEEPSEEK_API_KEY="your_key"; cargo run --example basic_usage

use ai_lib_rust::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Check if the required environment variable is set
    // Note: Protocol might look for specific env vars, but we check here for clarity
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("Warning: DEEPSEEK_API_KEY not set. This example might fail if the provider requires it.");
    }

    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("Hello! Explain the runtime briefly."),
    ];

    // Create client for DeepSeek chat
    let client = Provider::DeepSeek.model("deepseek-chat").build_client().await?;

    let resp = client
        .chat_completion(ChatCompletionRequest::new(messages).temperature(0.7).max_tokens(500))
        .await?;

    println!("Response:\n{}", resp.content);
    if let Some(usage) = resp.usage {
        println!("\nUsage: {usage:?}");
    }

    Ok(())
}
