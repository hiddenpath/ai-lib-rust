//! STT（语音转文字）模块：通过 Provider API（如 OpenAI Whisper）将音频转录为文本。

mod client;
mod types;

pub use client::{SttClient, SttClientBuilder};
pub use types::{SttOptions, Transcription, TranscriptionSegment};
