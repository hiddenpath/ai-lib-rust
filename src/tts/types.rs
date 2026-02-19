//! TTS (Text-to-Speech) types.

/// Audio output from TTS.
#[derive(Debug, Clone)]
pub struct AudioOutput {
    pub data: Vec<u8>,
    pub format: AudioFormat,
}

/// Supported audio formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Mp3,
    Opus,
    Aac,
    Flac,
    Wav,
    Pcm,
}

impl AudioFormat {
    pub fn mime_type(&self) -> &'static str {
        match self {
            Self::Mp3 => "audio/mpeg",
            Self::Opus => "audio/opus",
            Self::Aac => "audio/aac",
            Self::Flac => "audio/flac",
            Self::Wav => "audio/wav",
            Self::Pcm => "audio/pcm",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "opus" => Self::Opus,
            "aac" => Self::Aac,
            "flac" => Self::Flac,
            "wav" => Self::Wav,
            "pcm" => Self::Pcm,
            _ => Self::Mp3,
        }
    }
}

/// Options for TTS synthesis.
#[derive(Debug, Clone, Default)]
pub struct TtsOptions {
    pub voice: Option<String>,
    pub speed: Option<f32>,
    pub response_format: Option<String>,
}
