use ai_lib_rust::AiClient;

#[tokio::test]
async fn test_loading_all_providers() {
    // Set AI_PROTOCOL_DIR to the local path for testing
    let protocol_dir = "D:\\ai-protocol";
    std::env::set_var("AI_PROTOCOL_DIR", protocol_dir);

    let providers = vec!["openai", "anthropic", "gemini", "deepseek", "groq", "qwen"];

    for provider in providers {
        // Test direct provider loading via a model-id-like string
        let client = AiClient::new(&format!("{}/some-model", provider)).await;
        assert!(
            client.is_ok(),
            "Failed to load provider '{}': {:?}",
            provider,
            client.err()
        );
        let client = client.unwrap();
        assert_eq!(client.manifest.id, provider);
    }
}

#[tokio::test]
async fn test_loading_registered_models() {
    let protocol_dir = "D:\\ai-protocol";
    std::env::set_var("AI_PROTOCOL_DIR", protocol_dir);

    let models = vec![
        "openai/gpt-4o",
        "anthropic/claude-3-5-sonnet",
        "deepseek/deepseek-chat",
        "gemini/gemini-1.5-pro",
        "groq/llama3-70b-8192",
        "qwen/qwen-max",
    ];

    for model in models {
        let client = AiClient::new(model).await;
        assert!(
            client.is_ok(),
            "Failed to load registered model '{}': {:?}",
            model,
            client.err()
        );
    }
}
