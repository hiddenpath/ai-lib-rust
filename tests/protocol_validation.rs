//! Tests for protocol validation against JSON Schema

use ai_lib_rust::protocol::{ProtocolError, ProtocolLoader};

#[tokio::test]
async fn test_valid_provider_manifest() {
    let protocol_dir = std::env::var("AI_PROTOCOL_DIR")
        .or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        .unwrap_or_else(|_| "D:\\ai-protocol".to_string());
    std::env::set_var("AI_PROTOCOL_DIR", &protocol_dir);

    let loader = ProtocolLoader::new().with_base_path(&protocol_dir);

    // Test loading a known valid provider
    let result = loader.load_provider("openai").await;
    assert!(
        result.is_ok(),
        "OpenAI manifest should be valid: {:?}",
        result.err()
    );
}

#[tokio::test]
async fn test_invalid_yaml_structure() {
    let protocol_dir = std::env::var("AI_PROTOCOL_DIR")
        .or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        .unwrap_or_else(|_| "D:\\ai-protocol".to_string());
    std::env::set_var("AI_PROTOCOL_DIR", &protocol_dir);

    // Create a temporary invalid manifest file
    let temp_dir = std::env::temp_dir().join(format!(
        "ai-lib-rust-invalid-provider-{}",
        std::process::id()
    ));
    let providers_dir = temp_dir.join("v1").join("providers");
    std::fs::create_dir_all(&providers_dir).unwrap();
    let temp_file = providers_dir.join("invalid_provider.yaml");

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
    let _ = std::fs::remove_dir_all(&temp_dir);

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
    let temp_dir = std::env::temp_dir().join(format!(
        "ai-lib-rust-incomplete-provider-{}",
        std::process::id()
    ));
    let providers_dir = temp_dir.join("v1").join("providers");
    std::fs::create_dir_all(&providers_dir).unwrap();
    let temp_file = providers_dir.join("incomplete_provider.yaml");

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
    let _ = std::fs::remove_dir_all(&temp_dir);

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

#[tokio::test]
async fn test_loader_prefers_v2_provider_over_v1() {
    let temp_root = std::env::temp_dir().join(format!(
        "ai-lib-rust-protocol-prefers-v2-{}",
        std::process::id()
    ));
    let dst_v2 = temp_root
        .join("dist")
        .join("v2")
        .join("providers")
        .join("openai.json");
    let dst_v1 = temp_root
        .join("dist")
        .join("v1")
        .join("providers")
        .join("openai.json");
    std::fs::create_dir_all(dst_v2.parent().unwrap()).unwrap();
    std::fs::create_dir_all(dst_v1.parent().unwrap()).unwrap();
    std::fs::write(
        &dst_v2,
        r#"{
  "id":"openai",
  "protocol_version":"2.0",
  "status":"stable",
  "category":"ai_provider",
  "official_url":"https://example.com",
  "support_contact":"https://example.com/support",
  "endpoint":{"base_url":"https://v2.example.com"},
  "error_classification":{"by_http_status":{"429":"rate_limited"}},
  "capabilities":{"required":["text","streaming","tools"],"optional":[]},
  "capability_profile":{"phase":"ios_v1","inputs":{"modalities":["text"]}}
}"#,
    )
    .unwrap();
    std::fs::write(
        &dst_v1,
        r#"{
  "id":"openai",
  "protocol_version":"1.5",
  "status":"stable",
  "category":"ai_provider",
  "official_url":"https://example.com",
  "support_contact":"https://example.com/support",
  "endpoint":{"base_url":"https://v1.example.com"},
  "error_classification":{"by_http_status":{"429":"rate_limited"}},
  "capabilities":{"streaming":true,"tools":true,"vision":false}
}"#,
    )
    .unwrap();

    let loader = ProtocolLoader::new().with_base_path(&temp_root);
    let manifest = loader.load_provider("openai").await.unwrap();
    assert_eq!(manifest.protocol_version, "2.0");
    assert!(manifest.capability_profile.is_some());

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[tokio::test]
async fn test_loader_falls_back_to_v1_when_v2_missing() {
    let protocol_dir = std::env::var("AI_PROTOCOL_DIR")
        .or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        .unwrap_or_else(|_| "D:\\ai-protocol".to_string());
    let src_v1 = std::path::Path::new(&protocol_dir)
        .join("dist")
        .join("v1")
        .join("providers")
        .join("openai.json");
    if !src_v1.exists() {
        return;
    }

    let temp_root = std::env::temp_dir().join(format!(
        "ai-lib-rust-protocol-fallback-v1-{}",
        std::process::id()
    ));
    let dst_v1 = temp_root
        .join("dist")
        .join("v1")
        .join("providers")
        .join("openai.json");
    std::fs::create_dir_all(dst_v1.parent().unwrap()).unwrap();
    std::fs::copy(&src_v1, &dst_v1).unwrap();

    let loader = ProtocolLoader::new().with_base_path(&temp_root);
    let manifest = loader.load_provider("openai").await.unwrap();
    assert_eq!(manifest.protocol_version, "1.5");

    let _ = std::fs::remove_dir_all(&temp_root);
}
