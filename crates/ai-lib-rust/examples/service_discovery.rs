use ai_lib_rust::{AiClientBuilder, EndpointExt};
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. Initialize client with DeepSeek (supports get_balance)
    let client = AiClientBuilder::new()
        .protocol_path("../ai-protocol/v1/providers".to_string())
        .build("deepseek")
        .await?;

    println!("--- Service Discovery: DeepSeek ---");

    // 2. Call list_models (Standard Service)
    let models = EndpointExt::list_remote_models(&client).await?;
    println!("Available Models: {:?}", models);

    // 3. Call get_balance (Custom Service)
    match EndpointExt::call_service(&client, "get_balance").await {
        Ok(balance) => println!("Account Balance: {:#?}", balance),
        Err(e) => println!(
            "Could not retrieve balance: {} (Expected if API key is invalid)",
            e
        ),
    }

    // 4. OpenAI Service Discovery
    println!("\n--- Service Discovery: OpenAI ---");
    let openai_client = AiClientBuilder::new()
        .protocol_path("../ai-protocol/v1/providers".to_string())
        .build("openai")
        .await?;

    match EndpointExt::call_service(&openai_client, "get_usage").await {
        Ok(usage) => println!("Usage Data: {:#?}", usage),
        Err(e) => println!("Could not retrieve usage: {}", e),
    }

    Ok(())
}
