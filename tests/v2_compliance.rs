//! V2 协议跨模块集成测试 — 验证 Manifest→Driver→Registry→MCP→CU 全链路正确性
//!
//! V2 compliance integration tests validating the complete chain from manifest
//! loading through ProviderDriver selection, Capability Registry, MCP bridge,
//! and Computer Use safety policy enforcement.

#[cfg(all(feature = "mcp", feature = "computer_use", feature = "multimodal"))]
mod v2_integration {
    use ai_lib_rust::computer_use::{
        self, ComputerAction, ImplementationStyle, SafetyPolicy,
    };
    use ai_lib_rust::drivers::create_driver;
    use ai_lib_rust::mcp::{self, McpTool, McpToolBridge, McpToolResult, McpContent};
    use ai_lib_rust::multimodal::{
        Modality, MultimodalCapabilities, validate_content_modalities,
    };
    use ai_lib_rust::protocol::v2::capabilities::Capability;
    use ai_lib_rust::protocol::v2::manifest::{ApiStyle, ManifestV2};
    use ai_lib_rust::registry::CapabilityRegistry;
    use std::collections::HashMap;

    /// End-to-end: parse V2 manifest → create driver → verify capabilities.
    #[test]
    fn test_full_chain_openai() {
        let yaml = r#"
id: openai
protocol_version: "2.0"
name: OpenAI
status: stable
endpoint:
  base_url: https://api.openai.com/v1
  chat: /chat/completions
capabilities:
  required: [text, streaming, tools, mcp_client]
  optional: [vision, computer_use]
  feature_flags:
    structured_output: true
    parallel_tool_calls: true
streaming:
  decoder:
    format: sse
    strategy: openai_chat
mcp:
  client:
    supported: true
    protocol_version: "2025-11-25"
    transports: [streamable_http, sse]
    provider_mapping:
      tool_type: mcp
      config_method: tool_parameter
computer_use:
  supported: true
  status: preview
  implementation: screen_based
  safety:
    confirmation_required: true
    sandbox_mode: recommended
  provider_mapping:
    tool_type: computer_use_preview
multimodal:
  input:
    vision:
      supported: true
      formats: [jpeg, png, webp]
      encoding_methods: [base64_inline, url]
  output:
    text: true
    audio:
      supported: true
"#;
        // Step 1: Parse manifest
        let manifest: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        assert!(manifest.is_v2());
        assert_eq!(manifest.id, "openai");

        // Step 2: Detect API style → create driver
        let api_style = manifest.detect_api_style();
        assert_eq!(api_style, ApiStyle::OpenAiCompatible);
        let driver = create_driver(
            api_style,
            &manifest.id,
            vec![Capability::Text, Capability::Streaming, Capability::McpClient],
        );
        assert!(driver.supported_capabilities().contains(&Capability::McpClient));

        // Step 3: Registry — validate requirements
        let registry = CapabilityRegistry::from_capabilities(&manifest.capabilities);
        assert!(registry.validate_requirements().is_ok());
        assert!(registry.is_active(Capability::Text));
        assert!(registry.is_active(Capability::Streaming));

        // Step 4: MCP bridge — convert tools
        let bridge = McpToolBridge::new("filesystem");
        let mcp_tools = vec![
            McpTool {
                name: "read_file".into(),
                description: Some("Read a file".into()),
                input_schema: Some(serde_json::json!({"type": "object", "properties": {"path": {"type": "string"}}})),
            },
        ];
        let protocol_tools = bridge.mcp_tools_to_protocol(&mcp_tools);
        assert_eq!(protocol_tools.len(), 1);
        assert_eq!(protocol_tools[0].function.name, "mcp__filesystem__read_file");

        // Step 5: Provider config extraction
        let mcp_config = mcp::extract_provider_config(manifest.mcp.as_ref().unwrap());
        assert!(mcp_config.is_some());
        let mcp_pc = mcp_config.unwrap();
        assert_eq!(mcp_pc.tool_type, "mcp");

        // Step 6: Computer Use safety
        let cu_config = computer_use::extract_provider_config(manifest.computer_use.as_ref().unwrap());
        assert!(cu_config.is_some());
        assert_eq!(cu_config.unwrap().implementation, ImplementationStyle::ScreenBased);

        let safety = SafetyPolicy::from_config(manifest.computer_use.as_ref().unwrap());
        assert!(safety.confirmation_required);
        let action = ComputerAction::Screenshot { format: "png".into() };
        assert!(safety.validate_action(&action, 0).is_ok());

        // Step 7: Multimodal capabilities
        let mm_caps = MultimodalCapabilities::from_config(manifest.multimodal.as_ref().unwrap());
        assert!(mm_caps.supports_input(Modality::Image));
        assert!(mm_caps.supports_output(Modality::Audio));
        assert!(mm_caps.validate_image_format("jpeg"));
    }

    /// End-to-end: Anthropic — different API style, MCP beta header.
    #[test]
    fn test_full_chain_anthropic() {
        let yaml = r#"
id: anthropic
protocol_version: "2.0"
name: Anthropic
endpoint:
  base_url: https://api.anthropic.com/v1
  chat: /messages
capabilities:
  required: [text, streaming, tools, mcp_client, computer_use]
  optional: [vision, reasoning]
streaming:
  decoder:
    format: anthropic_sse
    strategy: anthropic_event_stream
mcp:
  client:
    supported: true
    transports: [sse]
    provider_mapping:
      beta_header: "mcp-client-2025-11-20"
      config_method: tool_parameter
computer_use:
  supported: true
  status: beta
  implementation: screen_based
  safety:
    confirmation_required: true
    sandbox_mode: recommended
    action_logging: true
  provider_mapping:
    tool_type: computer_20251124
    beta_header: "computer-use-2025-11-24"
"#;
        let manifest: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.detect_api_style(), ApiStyle::AnthropicMessages);

        let driver = create_driver(
            ApiStyle::AnthropicMessages,
            "anthropic",
            vec![Capability::Text, Capability::ComputerUse],
        );
        assert!(driver.supported_capabilities().contains(&Capability::ComputerUse));

        // MCP beta header
        let mcp_pc = mcp::extract_provider_config(manifest.mcp.as_ref().unwrap()).unwrap();
        assert_eq!(mcp_pc.beta_header.as_deref(), Some("mcp-client-2025-11-20"));

        // CU beta header
        let cu_pc = computer_use::extract_provider_config(manifest.computer_use.as_ref().unwrap()).unwrap();
        assert_eq!(cu_pc.tool_type, "computer_20251124");
        assert_eq!(cu_pc.beta_header.as_deref(), Some("computer-use-2025-11-24"));
    }

    /// End-to-end: Gemini — tool_based CU, SDK config for MCP.
    #[test]
    fn test_full_chain_gemini() {
        let yaml = r#"
id: google
protocol_version: "2.0"
name: Google Gemini
endpoint:
  base_url: https://generativelanguage.googleapis.com/v1beta
  chat: "/models/{model}:generateContent"
capabilities:
  required: [text, streaming, tools, computer_use]
  optional: [vision, audio, video]
mcp:
  client:
    supported: true
    transports: [streamable_http, sse]
    provider_mapping:
      config_method: sdk_config
computer_use:
  supported: true
  status: ga
  implementation: tool_based
  provider_mapping:
    tool_type: computer_use
    config_method: sdk_config
multimodal:
  input:
    vision:
      supported: true
      formats: [jpeg, png]
      encoding_methods: [base64_inline, url]
    audio:
      supported: true
      formats: [mp3, wav, flac]
    video:
      supported: true
      formats: [mp4, mov]
  output:
    text: true
"#;
        let manifest: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(manifest.detect_api_style(), ApiStyle::GeminiGenerate);

        // Gemini: tool_based CU
        let cu_pc = computer_use::extract_provider_config(manifest.computer_use.as_ref().unwrap()).unwrap();
        assert_eq!(cu_pc.implementation, ImplementationStyle::ToolBased);

        // Gemini: SDK config for MCP
        let mcp_pc = mcp::extract_provider_config(manifest.mcp.as_ref().unwrap()).unwrap();
        assert_eq!(mcp_pc.config_method, mcp::McpConfigMethod::SdkConfig);

        // Multimodal: supports video input
        let mm_caps = MultimodalCapabilities::from_config(manifest.multimodal.as_ref().unwrap());
        assert!(mm_caps.supports_input(Modality::Video));
        assert!(mm_caps.validate_video_format("mp4"));
        assert!(!mm_caps.validate_video_format("avi"));
    }

    /// MCP tool bridge round-trip: MCP → Protocol → MCP.
    #[test]
    fn test_mcp_bridge_roundtrip() {
        let bridge = McpToolBridge::new("testserver");
        let tool = McpTool {
            name: "calculate".into(),
            description: Some("Perform calculation".into()),
            input_schema: Some(serde_json::json!({"type": "object", "properties": {"expr": {"type": "string"}}})),
        };

        // Forward: MCP → Protocol
        let protocol_tools = bridge.mcp_tools_to_protocol(&[tool]);
        assert_eq!(protocol_tools[0].function.name, "mcp__testserver__calculate");

        // Reverse: Protocol call → MCP invocation
        let call = ai_lib_rust::types::tool::ToolCall {
            id: "call_1".into(),
            name: "mcp__testserver__calculate".into(),
            arguments: serde_json::json!({"expr": "2+2"}),
        };
        let invocation = bridge.protocol_call_to_mcp(&call).unwrap();
        assert_eq!(invocation.name, "calculate");
        assert_eq!(invocation.arguments["expr"], "2+2");

        // Result mapping
        let mcp_result = McpToolResult {
            content: vec![McpContent {
                content_type: "text".into(),
                text: Some("4".into()),
                extra: HashMap::new(),
            }],
            is_error: false,
        };
        let proto_result = bridge.mcp_result_to_protocol("call_1", &mcp_result);
        assert!(!proto_result.is_error);
    }

    /// CU safety: domain allowlist + max actions enforcement.
    #[test]
    fn test_cu_safety_enforcement() {
        let mut policy = SafetyPolicy::default();
        policy.max_actions_per_turn = 3;
        policy.domain_allowlist.insert("example.com".into());

        // Actions within limit
        let screenshot = ComputerAction::Screenshot { format: "png".into() };
        assert!(policy.validate_action(&screenshot, 0).is_ok());
        assert!(policy.validate_action(&screenshot, 1).is_ok());
        assert!(policy.validate_action(&screenshot, 2).is_ok());
        assert!(policy.validate_action(&screenshot, 3).is_err()); // 4th action blocked

        // Domain filtering
        let ok_nav = ComputerAction::BrowserNavigate { url: "https://example.com/page".into() };
        assert!(policy.validate_action(&ok_nav, 0).is_ok());

        let blocked_nav = ComputerAction::BrowserNavigate { url: "https://evil.com/phish".into() };
        assert!(policy.validate_action(&blocked_nav, 0).is_err());
    }

    /// Multimodal validation: reject unsupported modality.
    #[test]
    fn test_multimodal_validation_chain() {
        let yaml = r#"
id: test
protocol_version: "2.0"
endpoint:
  base_url: https://example.com
capabilities:
  required: [text, vision]
multimodal:
  input:
    vision:
      supported: true
      formats: [jpeg, png]
      encoding_methods: [base64_inline]
  output:
    text: true
"#;
        let manifest: ManifestV2 = serde_yaml::from_str(yaml).unwrap();
        let caps = MultimodalCapabilities::from_config(manifest.multimodal.as_ref().unwrap());

        // Valid: text + image
        let blocks = vec![
            serde_json::json!({"type": "text", "text": "Describe this"}),
            serde_json::json!({"type": "image", "source": {}}),
        ];
        assert!(validate_content_modalities(&blocks, &caps).is_ok());

        // Invalid: video not supported
        let blocks_video = vec![serde_json::json!({"type": "video", "source": {}})];
        let err = validate_content_modalities(&blocks_video, &caps).unwrap_err();
        assert!(err.contains(&Modality::Video));
    }
}
