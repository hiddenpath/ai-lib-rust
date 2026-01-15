//! Real DeepSeek streaming example (no mock).
//!
//! Prerequisites:
//! - Set `DEEPSEEK_API_KEY`
//! - Ensure `ai-protocol` provider manifests are available (see README paths)
//!
//! Run:
//!   DEEPSEEK_API_KEY=your_key cargo run --example deepseek_chat_stream

use ai_lib_rust::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    if std::env::var("DEEPSEEK_API_KEY").is_err() {
        eprintln!("Error: DEEPSEEK_API_KEY environment variable is not set.");
        eprintln!("Run with: DEEPSEEK_API_KEY=your_key cargo run --example deepseek_chat_stream");
        std::process::exit(1);
    }

    let messages = vec![
        Message::system("You are a helpful assistant."),
        Message::user("Say hello in one short sentence, then list two numbers."),
    ];

    let client = Provider::DeepSeek.model("deepseek-chat").build_client().await?;

    // Use facade: stream then collect into UnifiedResponse (developer-friendly).
    let resp = client
        .chat_completion(ChatCompletionRequest::new(messages).temperature(0.2).max_tokens(128).stream())
        .await?;

    println!("\n\n--- Content ---\n{}", resp.content);
    println!("\n--- Tool calls ---\n{:#?}", resp.tool_calls);
    println!("\n--- Usage ---\n{:#?}", resp.usage);

    Ok(())
}
