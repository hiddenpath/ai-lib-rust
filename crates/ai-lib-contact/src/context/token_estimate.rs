use ai_lib_core::types::message::{ContentBlock, Message, MessageContent, MessageRole};

/// Rough heuristic: 1 token ≈ 4 UTF-8 bytes (matches Eos SessionMirror / Phase 2 flat path).
pub const CHARS_PER_TOKEN: u32 = 4;

pub fn estimate_tokens(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    text.len().div_ceil(CHARS_PER_TOKEN as usize) as u32
}

pub fn estimate_message_tokens(message: &Message) -> u32 {
    let role_cost = estimate_tokens(match message.role {
        MessageRole::System => "system",
        MessageRole::User => "user",
        MessageRole::Assistant => "assistant",
        MessageRole::Tool => "tool",
    });
    let content_cost = match &message.content {
        MessageContent::Text(text) => estimate_tokens(text),
        MessageContent::Blocks(blocks) => blocks.iter().map(estimate_block_tokens).sum(),
    };
    role_cost + content_cost
}

fn estimate_block_tokens(block: &ContentBlock) -> u32 {
    match block {
        ContentBlock::Text { text } => estimate_tokens(text),
        ContentBlock::Image { source } => estimate_tokens(&source.data),
        ContentBlock::Audio { source } => estimate_tokens(&source.data),
        ContentBlock::ToolUse { id, name, input } => {
            estimate_tokens(id)
                + estimate_tokens(name)
                + estimate_tokens(&input.to_string())
        }
        ContentBlock::ToolResult {
            tool_use_id,
            content,
        } => estimate_tokens(tool_use_id) + estimate_tokens(&content.to_string()),
    }
}
