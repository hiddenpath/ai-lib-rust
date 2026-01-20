//! Pipeline interpreter layer - the core operator execution engine
//!
//! This module implements the operator pipeline that processes streaming responses
//! according to protocol configuration. The pipeline consists of:
//! - Decoder: Parses raw bytes into frames
//! - Transforms: A sequence of optional operators (Selector, Accumulator, FanOut, etc.)
//! - EventMapper: Converts frames to unified events

pub mod accumulate;
pub mod decode;
pub mod event_map;
pub mod fan_out;
pub mod select;

// Resilience Operators
pub mod fallback;
pub mod retry;

#[cfg(test)]
mod tests;

use crate::protocol::ProtocolManifest;
use crate::types::events::StreamingEvent;
use crate::{BoxStream, PipeResult};

/// Core transformer interface: all logic operators follow this unified abstraction
#[async_trait::async_trait]
pub trait Transform: Send + Sync {
    /// A transform takes a stream of JSON values and returns a new stream of JSON values
    async fn transform(
        &self,
        input: BoxStream<'static, serde_json::Value>,
    ) -> PipeResult<BoxStream<'static, serde_json::Value>>;
}

/// Specialized mapper for the final stage of the pipeline
#[async_trait::async_trait]
pub trait Mapper: Send + Sync {
    /// A mapper takes a stream of JSON values and returns a stream of unified events
    async fn map(
        &self,
        input: BoxStream<'static, serde_json::Value>,
    ) -> PipeResult<BoxStream<'static, StreamingEvent>>;
}

/// Decoder trait for stream decoding
#[async_trait::async_trait]
pub trait Decoder: Send + Sync {
    /// Decode a byte stream into JSON values
    async fn decode_stream(
        &self,
        input: BoxStream<'static, bytes::Bytes>,
    ) -> PipeResult<BoxStream<'static, serde_json::Value>>;
}

/// Pipeline error types
#[derive(Debug, thiserror::Error)]
pub enum PipelineError {
    #[error("Decoder error: {0}")]
    Decoder(String),

    #[error("Selector error: {0}")]
    Selector(String),

    #[error("Accumulator error: {0}")]
    Accumulator(String),

    #[error("Event mapper error: {0}")]
    EventMapper(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Missing required field: {name}{}", .hint.as_ref().map(|h| format!("\n💡 Hint: {}", h)).unwrap_or_default())]
    MissingField { name: String, hint: Option<String> },

    #[error("Invalid JSON path: {path} - {error}{}", .hint.as_ref().map(|h| format!("\n💡 Hint: {}", h)).unwrap_or_default())]
    InvalidJsonPath {
        path: String,
        error: String,
        hint: Option<String>,
    },

    #[error("Operator execution failed: {operator} - {reason}{}", .hint.as_ref().map(|h| format!("\n💡 Hint: {}", h)).unwrap_or_default())]
    Execution {
        operator: String,
        reason: String,
        hint: Option<String>,
    },
}

impl PipelineError {
    /// Attach an actionable hint to the error
    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        let hint_val = Some(hint.into());
        match self {
            PipelineError::MissingField { ref mut hint, .. } => *hint = hint_val,
            PipelineError::InvalidJsonPath { ref mut hint, .. } => *hint = hint_val,
            PipelineError::Execution { ref mut hint, .. } => *hint = hint_val,
            _ => (),
        }
        self
    }
}

/// Pipeline builder that constructs the operator chain from protocol manifest
pub struct PipelineBuilder {
    decoder: Option<Box<dyn Decoder>>,
    transforms: Vec<Box<dyn Transform>>,
    mapper: Option<Box<dyn Mapper>>,
}

impl PipelineBuilder {
    pub fn new() -> Self {
        Self {
            decoder: None,
            transforms: Vec::new(),
            mapper: None,
        }
    }

    pub fn set_decoder(mut self, decoder: Box<dyn Decoder>) -> Self {
        self.decoder = Some(decoder);
        self
    }

    pub fn add_transform(mut self, transform: Box<dyn Transform>) -> Self {
        self.transforms.push(transform);
        self
    }

    pub fn set_mapper(mut self, mapper: Box<dyn Mapper>) -> Self {
        self.mapper = Some(mapper);
        self
    }

    pub fn build(self) -> Result<Pipeline, PipelineError> {
        Ok(Pipeline {
            decoder: self
                .decoder
                .ok_or_else(|| PipelineError::Configuration("Decoder is required".to_string()))?,
            transforms: self.transforms,
            mapper: self.mapper.ok_or_else(|| {
                PipelineError::Configuration("Event mapper is required".to_string())
            })?,
        })
    }
}

/// Pipeline that processes streaming responses
pub struct Pipeline {
    decoder: Box<dyn Decoder>,
    transforms: Vec<Box<dyn Transform>>,
    mapper: Box<dyn Mapper>,
}

impl Pipeline {
    /// Create pipeline from protocol manifest
    pub fn from_manifest(manifest: &ProtocolManifest) -> Result<Self, PipelineError> {
        let mut builder = PipelineBuilder::new();

        if let Some(streaming) = &manifest.streaming {
            // 1. Build decoder
            if let Some(decoder_config) = &streaming.decoder {
                builder = builder.set_decoder(decode::create_decoder(decoder_config)?);
            } else {
                return Err(PipelineError::Configuration(
                    "streaming.decoder is required for streaming pipelines".to_string(),
                ));
            }

            // 2. Build transforms in order
            if let Some(frame_selector) = &streaming.frame_selector {
                builder = builder.add_transform(select::create_selector(frame_selector)?);
            }

            if let Some(accumulator_config) = &streaming.accumulator {
                builder =
                    builder.add_transform(accumulate::create_accumulator(accumulator_config)?);
            }

            if let Some(candidate_config) = &streaming.candidate {
                if candidate_config.fan_out.unwrap_or(false) {
                    builder = builder.add_transform(fan_out::create_fan_out(candidate_config)?);
                }
            }

            // 3. Build event mapper
            // Prefer manifest-driven rules. If none provided, fallback to adapter-based defaults.
            if !streaming.event_map.is_empty() {
                builder = builder.set_mapper(event_map::create_event_mapper(&streaming.event_map)?);
            } else {
                let tool_use = manifest.tooling.as_ref().and_then(|t| t.tool_use.clone());
                // Default: manifest-driven path mapping for OpenAI-compatible streaming
                builder = builder.set_mapper(Box::new(event_map::PathEventMapper::new(
                    streaming.content_path.clone(),
                    streaming.tool_call_path.clone(),
                    streaming.usage_path.clone(),
                    tool_use,
                )));
            }
        }

        builder.build()
    }

    /// Process a byte stream through the pipeline
    pub async fn process_stream(
        &self,
        input: BoxStream<'static, bytes::Bytes>,
    ) -> PipeResult<BoxStream<'static, StreamingEvent>> {
        // 1. Start with decoding: Bytes -> JSON Value
        let mut stream = self.decoder.decode_stream(input).await?;

        // 2. Apply all transforms in sequence: Value -> Value
        for transform in &self.transforms {
            stream = transform.transform(stream).await?;
        }

        // 3. Final mapping to events: Value -> Event
        let events = self.mapper.map(stream).await?;

        Ok(events)
    }

    pub async fn process_stream_arc(
        self: std::sync::Arc<Self>,
        input: BoxStream<'static, bytes::Bytes>,
    ) -> PipeResult<BoxStream<'static, StreamingEvent>> {
        self.process_stream(input).await
    }
}

impl Default for PipelineBuilder {
    fn default() -> Self {
        Self::new()
    }
}
