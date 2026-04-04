//! Basic usage example
//!
//! This example demonstrates how to use the unified client interface
//! to interact with AI models through the protocol runtime.
//!
//! API Keys are configured via environment variables:
//! - DEEPSEEK_API_KEY for Deepseek
//!
//! Usage (PowerShell):
//!   $env:DEEPSEEK_API_KEY="your_key"; cargo run --example basic_usage

use ai_lib_rust::{AiClient, Message};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Check if the required environment variable is set
    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("Warning: DEEPSEEK_API_KEY not set. This example might fail if the provider requires it.");
    }

    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("Hello!"),
    ];

    // Create client directly using provider/model string (protocol-driven)
    let client = AiClient::new("deepseek/deepseek-chat").await?;

    // Use the chat builder API
    let resp = client
        .chat()
        .messages(messages)
        .temperature(0.7)
        .max_tokens(500)
        .execute()
        .await?;

    println!("Response:\n{}", resp.content);
    if let Some(usage) = resp.usage {
        println!("\nUsage: {usage:?}");
    }

    Ok(())
}
