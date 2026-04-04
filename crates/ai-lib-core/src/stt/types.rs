//! STT (Speech-to-Text) types.

use serde::{Deserialize, Serialize};

/// Transcription result from STT.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcription {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub segments: Option<Vec<TranscriptionSegment>>,
}

/// A segment of transcribed text with timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptionSegment {
    pub start: f32,
    pub end: f32,
    pub text: String,
}

/// Options for STT transcription.
#[derive(Debug, Clone, Default)]
pub struct SttOptions {
    pub language: Option<String>,
    pub prompt: Option<String>,
    pub temperature: Option<f32>,
    pub response_format: Option<String>,
}
