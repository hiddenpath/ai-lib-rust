//! Unified message format based on AI-Protocol standard_schema

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Unified message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
    /// Required when role is Tool (OpenAI API: tool_call_id).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(text.into()),
            tool_call_id: None,
        }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(text.into()),
            tool_call_id: None,
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(text.into()),
            tool_call_id: None,
        }
    }

    /// Create a tool result message for multi-turn tool calling.
    ///
    /// OpenAI and similar APIs expect `role: "tool"` with `tool_call_id` and `content`.
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Tool,
            content: MessageContent::Text(content.into()),
            tool_call_id: Some(tool_call_id.into()),
        }
    }

    pub fn with_content(role: MessageRole, content: MessageContent) -> Self {
        Self {
            role,
            content,
            tool_call_id: None,
        }
    }

    pub fn contains_image(&self) -> bool {
        match &self.content {
            MessageContent::Text(_) => false,
            MessageContent::Blocks(bs) => {
                bs.iter().any(|b| matches!(b, ContentBlock::Image { .. }))
            }
        }
    }

    pub fn contains_audio(&self) -> bool {
        match &self.content {
            MessageContent::Text(_) => false,
            MessageContent::Blocks(bs) => {
                bs.iter().any(|b| matches!(b, ContentBlock::Audio { .. }))
            }
        }
    }
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    /// Tool result message (OpenAI API: role "tool").
    Tool,
}

/// Message content (can be string or array of content blocks)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}

impl MessageContent {
    pub fn text(text: impl Into<String>) -> Self {
        MessageContent::Text(text.into())
    }

    pub fn blocks(blocks: Vec<ContentBlock>) -> Self {
        MessageContent::Blocks(blocks)
    }
}

/// Content block (for multimodal or tool results)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "audio")]
    Audio { source: AudioSource },
    #[serde(rename = "tool_use")]
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        tool_use_id: String,
        content: serde_json::Value,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub data: String, // base64 encoded or URL
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,
    pub data: String, // base64 encoded or URL
}

impl ContentBlock {
    pub fn text(text: impl Into<String>) -> Self {
        ContentBlock::Text { text: text.into() }
    }

    pub fn image_base64(data: String, media_type: Option<String>) -> Self {
        ContentBlock::Image {
            source: ImageSource {
                source_type: "base64".to_string(),
                media_type,
                data,
            },
        }
    }

    pub fn audio_base64(data: String, media_type: Option<String>) -> Self {
        ContentBlock::Audio {
            source: AudioSource {
                source_type: "base64".to_string(),
                media_type,
                data,
            },
        }
    }

    pub fn image_from_file(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;
        let media_type = guess_media_type(path);
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(Self::image_base64(data, media_type))
    }

    pub fn audio_from_file(path: impl AsRef<Path>) -> crate::Result<Self> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;
        let media_type = guess_media_type(path);
        let data = base64::engine::general_purpose::STANDARD.encode(bytes);
        Ok(Self::audio_base64(data, media_type))
    }
}

fn guess_media_type(path: &Path) -> Option<String> {
    let ext = path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    let mt = match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        _ => return None,
    };
    Some(mt.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_tool() {
        let msg = Message::tool("call_abc123", "42");
        assert!(matches!(msg.role, MessageRole::Tool));
        assert_eq!(msg.tool_call_id.as_deref(), Some("call_abc123"));
        if let MessageContent::Text(s) = msg.content {
            assert_eq!(s, "42");
        } else {
            panic!("expected Text content");
        }
    }

    #[test]
    fn test_message_role_serialization() {
        let msg = Message::tool("call_xyz", "result");
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "tool");
        assert_eq!(json["content"], "result");
        assert_eq!(json["tool_call_id"], "call_xyz");
    }
}
