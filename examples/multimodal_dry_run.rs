use ai_lib_rust::{AiClient, Message, MessageRole};
use ai_lib_rust::types::message::{MessageContent, ContentBlock};
use ai_lib_rust::protocol::UnifiedRequest;

fn fake_b64() -> String {
    // Intentionally tiny placeholder payload for examples.
    // This is NOT a valid image/audio; it exists only to show the API surface and compile path.
    "AA==".to_string()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tip: point to your local ai-protocol checkout:
    //   - PowerShell: $env:AI_PROTOCOL_DIR="D:\ai-protocol"
    //
    // This example is designed to be safe without API keys:
    // - For protocols that do NOT declare multimodal capability, we trigger a fail-fast validation
    //   error before any network I/O.
    // - For protocols that DO declare capability, we only "dry-run" compile the request and print it.

    let image_msg = Message::with_content(
        MessageRole::User,
        MessageContent::blocks(vec![
            ContentBlock::text("Describe this image in one sentence."),
            ContentBlock::image_base64(fake_b64(), Some("image/png".to_string())),
        ]),
    );

    let audio_msg = Message::with_content(
        MessageRole::User,
        MessageContent::blocks(vec![
            ContentBlock::text("Transcribe this short audio."),
            ContentBlock::audio_base64(fake_b64(), Some("audio/wav".to_string())),
        ]),
    );

    // 1) Demonstrate fail-fast capability validation (no network):
    // DeepSeek does not declare vision/multimodal, so this should error immediately.
    let deepseek = AiClient::new("deepseek/deepseek-chat").await?;
    let deepseek_res = deepseek
        .chat()
        .messages(vec![Message::system("You are a helpful assistant."), image_msg.clone()])
        .execute()
        .await;
    println!("DeepSeek image request result: {deepseek_res:?}");

    // OpenAI declares vision but not audio (in current manifests), so audio should fail-fast.
    let openai = AiClient::new("openai/gpt-4o").await?;
    let openai_audio_res = openai
        .chat()
        .messages(vec![Message::system("You are a helpful assistant."), audio_msg.clone()])
        .execute()
        .await;
    println!("OpenAI audio request result: {openai_audio_res:?}");

    // 2) Dry-run compile requests for protocols that declare capabilities:
    // OpenAI: image-only
    let openai_unified = UnifiedRequest {
        operation: "chat".to_string(),
        model: "gpt-4o".to_string(),
        messages: vec![Message::system("You are a helpful assistant."), image_msg.clone()],
        temperature: None,
        max_tokens: Some(128),
        stream: false,
        tools: None,
        tool_choice: None,
    };
    let openai_compiled = openai.manifest.compile_request(&openai_unified)?;
    println!(
        "OpenAI compiled request (dry-run, AI-Protocol shape):\n{}",
        serde_json::to_string_pretty(&openai_compiled)?
    );

    // Gemini: image + audio (multimodal)
    let gemini = AiClient::new("gemini/gemini-1.5-pro").await?;
    let gemini_unified = UnifiedRequest {
        operation: "chat".to_string(),
        model: "gemini-1.5-pro".to_string(),
        messages: vec![
            Message::system("You are a helpful assistant."),
            image_msg,
            audio_msg,
        ],
        temperature: None,
        max_tokens: Some(128),
        stream: false,
        tools: None,
        tool_choice: None,
    };
    let gemini_compiled = gemini.manifest.compile_request(&gemini_unified)?;
    println!(
        "Gemini compiled request (dry-run, AI-Protocol shape):\n{}",
        serde_json::to_string_pretty(&gemini_compiled)?
    );

    Ok(())
}

