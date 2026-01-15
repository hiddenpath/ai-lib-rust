//! Standalone binary to validate all protocol manifests against JSON Schema.
//! Used by CI to ensure protocol files are valid.

use ai_lib_rust::protocol::ProtocolLoader;
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protocol_dir = std::env::var("AI_PROTOCOL_DIR")
        .or_else(|_| std::env::var("AI_PROTOCOL_PATH"))
        .unwrap_or_else(|_| {
            // Try common locations
            let candidates = vec![
                "ai-protocol",
                "../ai-protocol",
                "../../ai-protocol",
                "d:\\ai-protocol",
            ];
            for candidate in candidates {
                let path = PathBuf::from(candidate).join("v1").join("providers");
                if path.exists() {
                    return candidate.to_string();
                }
            }
            panic!("AI_PROTOCOL_DIR not set and no protocol directory found");
        });

    println!("Using protocol directory: {}", protocol_dir);
    std::env::set_var("AI_PROTOCOL_DIR", &protocol_dir);

    let loader = ProtocolLoader::new().with_base_path(&protocol_dir);

    // Validate all provider manifests
    let providers = vec![
        "openai", "anthropic", "gemini", "deepseek", "groq", "qwen",
    ];

    let mut errors = Vec::new();

    println!("\n=== Validating Provider Manifests ===");
    for provider in &providers {
        print!("Validating {}... ", provider);
        match loader.load_provider(provider).await {
            Ok(_) => println!("✅"),
            Err(e) => {
                println!("❌");
                errors.push(format!("  {}: {}", provider, e));
            }
        }
    }

    // Validate model registries
    println!("\n=== Validating Model Registries ===");
    let models_dir = PathBuf::from(&protocol_dir).join("v1").join("models");
    if models_dir.exists() {
        let model_files = std::fs::read_dir(&models_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "yaml" || ext == "yml")
                    .unwrap_or(false)
            })
            .collect::<Vec<_>>();

        for entry in model_files {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            print!("Validating model registry {}... ", file_name_str);

            // Try to load as a model registry (this will validate structure)
            // For now, we just check if it's valid YAML and can be parsed
            match std::fs::read_to_string(entry.path()) {
                Ok(content) => {
                    match serde_yaml::from_str::<serde_json::Value>(&content) {
                        Ok(_) => println!("✅"),
                        Err(e) => {
                            println!("❌");
                            errors.push(format!("  {}: Invalid YAML: {}", file_name_str, e));
                        }
                    }
                }
                Err(e) => {
                    println!("❌");
                    errors.push(format!("  {}: Read error: {}", file_name_str, e));
                }
            }
        }
    } else {
        println!("Models directory not found, skipping model validation");
    }

    // Summary
    println!("\n=== Summary ===");
    if errors.is_empty() {
        println!("✅ All protocol files are valid!");
        Ok(())
    } else {
        println!("❌ Found {} validation error(s):\n", errors.len());
        for err in &errors {
            println!("{}", err);
        }
        std::process::exit(1);
    }
}
