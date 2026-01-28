//! Unified message format based on AI-Protocol standard_schema

use base64::Engine as _;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Unified message structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: MessageContent,
}

impl Message {
    pub fn system(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: MessageContent::Text(text.into()),
        }
    }

    pub fn with_content(role: MessageRole, content: MessageContent) -> Self {
        Self { role, content }
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
