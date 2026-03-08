//! Generative manifest consumption tests.
//!
//! Verifies ai-lib-rust can parse and utilize latest ai-protocol V2 provider
//! manifests for generative/multimodal capabilities.
#![cfg(feature = "multimodal")]

use ai_lib_rust::multimodal::{Modality, MultimodalCapabilities};
use ai_lib_rust::protocol::v2::manifest::{ApiStyle, ManifestV2};
use std::fs;
use std::path::PathBuf;

fn resolve_ai_protocol_root() -> PathBuf {
    if let Ok(path) = std::env::var("AI_PROTOCOL_DIR") {
        return PathBuf::from(path);
    }
    if let Ok(path) = std::env::var("AI_PROTOCOL_PATH") {
        return PathBuf::from(path);
    }
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../ai-protocol"),
        manifest_dir.join("../../ai-protocol"),
        PathBuf::from("d:/ai-protocol"),
    ];
    for candidate in candidates {
        if candidate.exists() {
            return candidate;
        }
    }
    panic!("Unable to locate ai-protocol root for manifest consumption test");
}

#[test]
fn consume_latest_v2_generative_manifests() {
    let root = resolve_ai_protocol_root();
    let providers = ["google", "deepseek", "qwen", "doubao"];

    for provider in providers {
        let path = root.join(format!("v2/providers/{provider}.yaml"));
        let raw = fs::read_to_string(&path).unwrap_or_else(|e| {
            panic!("failed reading {}: {e}", path.display());
        });
        let manifest: ManifestV2 = serde_yaml::from_str(&raw).unwrap_or_else(|e| {
            panic!("failed parsing {}: {e}", path.display());
        });

        assert!(manifest.is_v2(), "{provider} should be parsed as V2");
        assert_eq!(manifest.id, provider);

        if provider == "google" {
            assert_eq!(manifest.detect_api_style(), ApiStyle::GeminiGenerate);
        } else {
            assert_eq!(manifest.detect_api_style(), ApiStyle::OpenAiCompatible);
        }

        let multimodal = manifest.multimodal.as_ref().expect("multimodal section required");
        let caps = MultimodalCapabilities::from_config(multimodal);

        assert!(caps.supports_input(Modality::Text));
        assert!(caps.supports_output(Modality::Text));
        if provider == "qwen" || provider == "google" {
            assert!(caps.supports_input(Modality::Video), "{provider} should support video input");
        }

        // Latest schema includes output.video declaration; runtimes must not drop it.
        let output_video_supported = multimodal
            .output
            .as_ref()
            .and_then(|o| o.video.as_ref())
            .map(|v| v.supported)
            .unwrap_or(false);
        assert!(!output_video_supported, "{provider} output.video expected false in current P0 manifests");
    }
}

#[test]
fn supports_structured_endpoint_chat_shape() {
    let raw = r#"
id: shape-compat
protocol_version: "2.0"
endpoint:
  base_url: "https://example.com"
  chat:
    path: "/v2/chat"
    method: "POST"
capabilities:
  required: ["text"]
  optional: []
"#;
    let manifest: ManifestV2 = serde_yaml::from_str(raw).expect("manifest should parse");
    assert_eq!(manifest.chat_path(), "/v2/chat");
    assert_eq!(manifest.base_url(), "https://example.com");
}
