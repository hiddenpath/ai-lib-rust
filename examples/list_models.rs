use ai_lib_rust::{AiClient, EndpointExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize client for a provider (we use DeepSeek as an example)
    println!("Initializing client for DeepSeek...");
    let client = AiClient::new("deepseek/deepseek-chat").await?;

    println!("Requesting model list from provider...");
    match EndpointExt::list_remote_models(&client).await {
        Ok(models) => {
            println!("Available models:");
            for model in models {
                println!("- {}", model);
            }
        }
        Err(e) => {
            println!("Error listing models: {}", e);
            println!("Note: This is expected if DEEPSEEK_API_KEY is missing or invalid.");
        }
    }

    Ok(())
}
