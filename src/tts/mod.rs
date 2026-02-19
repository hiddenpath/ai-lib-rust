//! TTS（文字转语音）模块：通过 Provider API（如 OpenAI TTS）将文本合成为音频。

mod client;
mod types;

pub use client::{TtsClient, TtsClientBuilder};
pub use types::{AudioFormat, AudioOutput, TtsOptions};
