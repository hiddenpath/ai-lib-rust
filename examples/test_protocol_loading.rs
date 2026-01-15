use ai_lib_rust::AiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test protocol loading
    let _client = AiClient::new("anthropic/claude-3-5-sonnet").await?;

    println!("✅ Successfully loaded Anthropic protocol");
    // println!("Provider ID: {}", client.manifest.id);
    // println!("Base URL: {}", client.manifest.base_url);
    // println!("Capabilities: {:?}", client.manifest.capabilities);

    // Test OpenAI protocol loading
    let _openai_client = AiClient::new("openai/gpt-4").await?;
    println!("✅ Successfully loaded OpenAI protocol");
    // println!("Provider ID: {}", openai_client.manifest.id);

    // Test Groq protocol loading (OpenAI-compatible)
    let _groq_client = AiClient::new("groq/llama-3.1-70b").await?;
    println!("✅ Successfully loaded Groq protocol");

    // Test Qwen protocol loading (DashScope OpenAI-compatible mode)
    let _qwen_client = AiClient::new("qwen/qwen-turbo").await?;
    println!("✅ Successfully loaded Qwen protocol");

    // Test Gemini protocol loading
    let _gemini_client = AiClient::new("gemini/gemini-1.5-pro").await?;
    println!("✅ Successfully loaded Gemini protocol");

    Ok(())
}
