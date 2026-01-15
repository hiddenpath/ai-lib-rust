//! Custom protocol example
//!
//! This example demonstrates how to load and use custom protocol configurations
//! from local files or remote URLs.

use ai_lib_rust::protocol::ProtocolLoader;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create a loader with custom base path
    let loader = ProtocolLoader::new()
        .with_base_path("./ai-protocol")
        .with_hot_reload(true);

    // Load a provider configuration
    let manifest = loader.load_provider("openai").await?;

    println!("Loaded protocol: {}", manifest.id);
    println!("Protocol version: {}", manifest.protocol_version);
    println!("Base URL: {}", manifest.base_url);
    println!("Capabilities: {:?}", manifest.capabilities);

    // The manifest can now be used to create a client or process requests
    // This demonstrates the protocol-driven architecture where all logic
    // is derived from the YAML configuration.

    Ok(())
}
