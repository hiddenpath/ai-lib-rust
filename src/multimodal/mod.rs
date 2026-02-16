//! 扩展多模态处理模块 — 提供跨厂商的输入/输出模态验证与格式转换
//!
//! Extended multimodal processing module for AI-Protocol V2.
//! Provides:
//! - Content format validation against manifest capabilities
//! - Provider-specific content formatting helpers
//! - Input modality detection and validation
//! - Output modality negotiation

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::protocol::v2::manifest::MultimodalConfig;

/// Supported input/output modality types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Modality {
    Text,
    Image,
    Audio,
    Video,
}

impl Modality {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Audio => "audio",
            Self::Video => "video",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "text" => Some(Self::Text),
            "image" => Some(Self::Image),
            "audio" => Some(Self::Audio),
            "video" => Some(Self::Video),
            _ => None,
        }
    }
}

/// Describes the multimodal capabilities of a provider, derived from the manifest.
#[derive(Debug, Clone)]
pub struct MultimodalCapabilities {
    pub input_modalities: HashSet<Modality>,
    pub output_modalities: HashSet<Modality>,
    pub image_formats: Vec<String>,
    pub audio_formats: Vec<String>,
    pub video_formats: Vec<String>,
    pub max_image_size: Option<String>,
    pub max_audio_duration: Option<String>,
    pub supports_omni: bool,
    pub supports_realtime_voice: bool,
}

impl MultimodalCapabilities {
    /// Build from a V2 multimodal config section.
    pub fn from_config(config: &MultimodalConfig) -> Self {
        let mut input_modalities = HashSet::new();
        let mut output_modalities = HashSet::new();
        let mut image_formats = Vec::new();
        let mut audio_formats = Vec::new();
        let mut video_formats = Vec::new();
        let mut max_image_size = None;
        let mut max_audio_duration = None;

        input_modalities.insert(Modality::Text);
        output_modalities.insert(Modality::Text);

        if let Some(input) = &config.input {
            if let Some(vision) = &input.vision {
                if vision.supported {
                    input_modalities.insert(Modality::Image);
                    image_formats = vision.formats.clone();
                    max_image_size = vision.max_file_size.clone();
                }
            }
            if let Some(audio) = &input.audio {
                if audio.supported {
                    input_modalities.insert(Modality::Audio);
                    audio_formats = audio.formats.clone();
                }
            }
            if let Some(video) = &input.video {
                if video.supported {
                    input_modalities.insert(Modality::Video);
                    video_formats = video.formats.clone();
                    max_audio_duration.clone_from(&video.formats.first().map(|_| "".to_string()));
                }
            }
        }

        if let Some(output) = &config.output {
            if let Some(audio_out) = &output.audio {
                if audio_out.supported {
                    output_modalities.insert(Modality::Audio);
                }
            }
            if let Some(image_out) = &output.image {
                if image_out.supported {
                    output_modalities.insert(Modality::Image);
                }
            }
        }

        let supports_omni = config
            .omni_mode
            .as_ref()
            .map(|o| o.supported)
            .unwrap_or(false);
        let supports_realtime_voice = config
            .omni_mode
            .as_ref()
            .map(|o| o.real_time_voice_chat)
            .unwrap_or(false);

        Self {
            input_modalities,
            output_modalities,
            image_formats,
            audio_formats,
            video_formats,
            max_image_size,
            max_audio_duration,
            supports_omni,
            supports_realtime_voice,
        }
    }

    /// Check if a given input modality is supported.
    pub fn supports_input(&self, modality: Modality) -> bool {
        self.input_modalities.contains(&modality)
    }

    /// Check if a given output modality is supported.
    pub fn supports_output(&self, modality: Modality) -> bool {
        self.output_modalities.contains(&modality)
    }

    /// Validate an image format against supported formats.
    pub fn validate_image_format(&self, format: &str) -> bool {
        if self.image_formats.is_empty() {
            return true; // No restrictions declared
        }
        self.image_formats.iter().any(|f| f.eq_ignore_ascii_case(format))
    }

    /// Validate an audio format against supported formats.
    pub fn validate_audio_format(&self, format: &str) -> bool {
        if self.audio_formats.is_empty() {
            return true;
        }
        self.audio_formats.iter().any(|f| f.eq_ignore_ascii_case(format))
    }

    /// Validate a video format against supported formats.
    pub fn validate_video_format(&self, format: &str) -> bool {
        if self.video_formats.is_empty() {
            return true;
        }
        self.video_formats.iter().any(|f| f.eq_ignore_ascii_case(format))
    }
}

/// Detect the modalities present in a list of content blocks.
pub fn detect_modalities(content_blocks: &[serde_json::Value]) -> HashSet<Modality> {
    let mut modalities = HashSet::new();
    for block in content_blocks {
        if let Some(block_type) = block.get("type").and_then(|t| t.as_str()) {
            match block_type {
                "text" => { modalities.insert(Modality::Text); }
                "image" | "image_url" => { modalities.insert(Modality::Image); }
                "audio" | "input_audio" => { modalities.insert(Modality::Audio); }
                "video" => { modalities.insert(Modality::Video); }
                _ => {}
            }
        }
    }
    if modalities.is_empty() {
        modalities.insert(Modality::Text);
    }
    modalities
}

/// Validate that all modalities in content blocks are supported by the provider.
pub fn validate_content_modalities(
    blocks: &[serde_json::Value],
    caps: &MultimodalCapabilities,
) -> Result<(), Vec<Modality>> {
    let detected = detect_modalities(blocks);
    let unsupported: Vec<Modality> = detected
        .into_iter()
        .filter(|m| !caps.supports_input(*m))
        .collect();
    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(unsupported)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::v2::manifest::*;

    fn make_config() -> MultimodalConfig {
        MultimodalConfig {
            input: Some(MultimodalInput {
                vision: Some(VisionConfig {
                    supported: true,
                    formats: vec!["jpeg".into(), "png".into(), "webp".into()],
                    encoding_methods: vec!["base64_inline".into(), "url".into()],
                    document_understanding: false,
                    max_file_size: Some("20MB".into()),
                    max_resolution: None,
                }),
                audio: Some(AudioInputConfig {
                    supported: true,
                    formats: vec!["mp3".into(), "wav".into()],
                    real_time_streaming: false,
                    speech_recognition: true,
                }),
                video: None,
            }),
            output: Some(MultimodalOutput {
                text: true,
                audio: Some(AudioOutputConfig {
                    supported: true,
                    real_time_tts: false,
                    natural_voice: true,
                    voice_selection: true,
                }),
                image: None,
            }),
            omni_mode: None,
        }
    }

    #[test]
    fn test_from_config() {
        let caps = MultimodalCapabilities::from_config(&make_config());
        assert!(caps.supports_input(Modality::Text));
        assert!(caps.supports_input(Modality::Image));
        assert!(caps.supports_input(Modality::Audio));
        assert!(!caps.supports_input(Modality::Video));
        assert!(caps.supports_output(Modality::Audio));
        assert!(!caps.supports_output(Modality::Image));
    }

    #[test]
    fn test_validate_image_format() {
        let caps = MultimodalCapabilities::from_config(&make_config());
        assert!(caps.validate_image_format("jpeg"));
        assert!(caps.validate_image_format("PNG")); // case insensitive
        assert!(!caps.validate_image_format("bmp"));
    }

    #[test]
    fn test_validate_audio_format() {
        let caps = MultimodalCapabilities::from_config(&make_config());
        assert!(caps.validate_audio_format("mp3"));
        assert!(!caps.validate_audio_format("flac"));
    }

    #[test]
    fn test_detect_modalities() {
        let blocks = vec![
            serde_json::json!({"type": "text", "text": "Hello"}),
            serde_json::json!({"type": "image", "source": {}}),
        ];
        let mods = detect_modalities(&blocks);
        assert!(mods.contains(&Modality::Text));
        assert!(mods.contains(&Modality::Image));
        assert!(!mods.contains(&Modality::Audio));
    }

    #[test]
    fn test_validate_content_modalities_ok() {
        let caps = MultimodalCapabilities::from_config(&make_config());
        let blocks = vec![
            serde_json::json!({"type": "text", "text": "Describe this image"}),
            serde_json::json!({"type": "image", "source": {"type": "url", "data": "http://..."}}),
        ];
        assert!(validate_content_modalities(&blocks, &caps).is_ok());
    }

    #[test]
    fn test_validate_content_modalities_fail() {
        let caps = MultimodalCapabilities::from_config(&make_config());
        let blocks = vec![
            serde_json::json!({"type": "video", "source": {}}),
        ];
        let err = validate_content_modalities(&blocks, &caps).unwrap_err();
        assert!(err.contains(&Modality::Video));
    }
}
