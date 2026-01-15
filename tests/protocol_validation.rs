//! Tests for protocol validation against JSON Schema

use ai_lib_rust::protocol::{ProtocolError, ProtocolLoader, ProtocolManifest};
use serde_yaml::Value;

#[tokio::test]
async fn test_valid_provider_manifest() {
    let protocol_dir = std::env::var("AI_PROTOCOL_DIR")
        .or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        .unwrap_or_else(|_| "D:\\ai-protocol".to_string());
    std::env::set_var("AI_PROTOCOL_DIR", &protocol_dir);

    let loader = ProtocolLoader::new().with_base_path(&protocol_dir);

    // Test loading a known valid provider
    let result = loader.load_provider("openai").await;
    assert!(result.is_ok(), "OpenAI manifest should be valid: {:?}", result.err());
}

#[tokio::test]
async fn test_invalid_yaml_structure() {
    let protocol_dir = std::env::var("AI_PROTOCOL_DIR")
        .or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        .unwrap_or_else(|_| "D:\\ai-protocol".to_string());
    std::env::set_var("AI_PROTOCOL_DIR", &protocol_dir);

    let loader = ProtocolLoader::new().with_base_path(&protocol_dir);

    // Create a temporary invalid manifest file
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("invalid_provider.yaml");

    // Write invalid YAML (missing required fields)
    std::fs::write(
        &temp_file,
        r#"
id: test-provider
# Missing protocol_version, base_url, etc.
"#,
    )
    .unwrap();

    // Try to load it (should fail validation)
    let temp_loader = ProtocolLoader::new().with_base_path(&temp_dir);
    let result = temp_loader.load_provider("invalid_provider").await;

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);

    // Should fail validation
    assert!(result.is_err(), "Invalid manifest should fail validation");
    if let Err(ProtocolError::ValidationError(_)) = result {
        // Expected error type
    } else {
        panic!("Expected ValidationError, got: {:?}", result.err());
    }
}

#[tokio::test]
async fn test_missing_required_fields() {
    let temp_dir = std::env::temp_dir();
    let temp_file = temp_dir.join("incomplete_provider.yaml");

    // Write manifest missing required fields
    std::fs::write(
        &temp_file,
        r#"
id: incomplete-provider
protocol_version: "1.1"
# Missing base_url
"#,
    )
    .unwrap();

    let loader = ProtocolLoader::new().with_base_path(&temp_dir);
    let result = loader.load_provider("incomplete_provider").await;

    // Cleanup
    let _ = std::fs::remove_file(&temp_file);

    assert!(result.is_err(), "Manifest with missing fields should fail");
}

#[tokio::test]
async fn test_all_providers_valid() {
    let protocol_dir = std::env::var("AI_PROTOCOL_DIR")
        .or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        .unwrap_or_else(|_| "D:\\ai-protocol".to_string());
    std::env::set_var("AI_PROTOCOL_DIR", &protocol_dir);

    let loader = ProtocolLoader::new().with_base_path(&protocol_dir);

    let providers = vec!["openai", "anthropic", "gemini", "deepseek", "groq", "qwen"];

    for provider in providers {
        let result = loader.load_provider(provider).await;
        assert!(
            result.is_ok(),
            "Provider '{}' should have valid manifest: {:?}",
            provider,
            result.err()
        );
    }
}
