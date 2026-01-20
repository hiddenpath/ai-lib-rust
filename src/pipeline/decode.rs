//! Streaming decoders (Bytes -> JSON Value)
//!
//! This module intentionally keeps provider logic out of code: it decodes *formats*
//! (SSE, NDJSON, etc.) based on manifest configuration.

use crate::pipeline::{Decoder, PipelineError};
use crate::protocol::DecoderConfig;
use crate::{BoxStream, PipeResult};
use bytes::Bytes;
use futures::{stream, StreamExt};
use serde_json::Value;

/// A minimal, manifest-driven SSE decoder:
/// - splits by delimiter (default "\n\n")
/// - strips `prefix` (default "data: ")
/// - stops on `done_signal` (default "[DONE]")
pub struct SseDecoder {
    delimiter: String,
    prefix: String,
    done_signal: String,
}

impl SseDecoder {
    pub fn new(
        delimiter: Option<String>,
        prefix: Option<String>,
        done_signal: Option<String>,
    ) -> Self {
        Self {
            delimiter: delimiter.unwrap_or_else(|| "\n\n".to_string()),
            prefix: prefix.unwrap_or_else(|| "data: ".to_string()),
            done_signal: done_signal.unwrap_or_else(|| "[DONE]".to_string()),
        }
    }

    pub fn from_config(cfg: &DecoderConfig) -> Result<Self, PipelineError> {
        Ok(Self::new(
            cfg.delimiter.clone(),
            cfg.prefix.clone(),
            cfg.done_signal.clone(),
        ))
    }

    // NOTE: Parsing is implemented inside `decode_stream()` so we can construct streams that do not
    // borrow `&self` (required to return `'static` streams for higher-level retry/fallback).
}

#[async_trait::async_trait]
impl Decoder for SseDecoder {
    async fn decode_stream(
        &self,
        input: BoxStream<'static, Bytes>,
    ) -> PipeResult<BoxStream<'static, Value>> {
        let delimiter = self.delimiter.clone();
        let delimiter_len = delimiter.len();
        let prefix = self.prefix.clone();
        let done_signal = self.done_signal.clone();

        // Incrementally buffer bytes and emit full frames split by delimiter.
        let stream = stream::unfold((input, String::new()), move |(mut input, mut buf)| {
            let delimiter = delimiter.clone();
            let prefix = prefix.clone();
            let done_signal = done_signal.clone();
            async move {
                let is_done = |s: &str| -> bool {
                    let t = s.trim();
                    t == done_signal
                        || t == format!("data: {}", done_signal)
                        || t == format!("data:{}", done_signal)
                };

                let parse_payload = |raw: &str| -> Option<Value> {
                    let trimmed = raw.trim();
                    if trimmed.is_empty() || is_done(trimmed) {
                        return None;
                    }

                    // Ignore SSE comment lines
                    if trimmed.starts_with(':') {
                        return None;
                    }

                    // Strip prefix if present
                    let payload = if trimmed.starts_with(&prefix) {
                        &trimmed[prefix.len()..]
                    } else if trimmed.starts_with("data:") {
                        trimmed[5..].trim_start()
                    } else {
                        trimmed
                    };

                    serde_json::from_str(payload).ok()
                };

                loop {
                    // If we have a full frame in buffer, emit it.
                    if let Some(idx) = buf.find(&delimiter) {
                        let frame = buf[..idx].to_string();
                        let rest_start = idx + delimiter_len;
                        buf = if rest_start <= buf.len() {
                            buf[rest_start..].to_string()
                        } else {
                            String::new()
                        };

                        if is_done(&frame) {
                            return None;
                        }

                        if let Some(v) = parse_payload(&frame) {
                            return Some((Ok(v), (input, buf)));
                        }

                        // Skip non-json frames; keep looping.
                        continue;
                    }

                    // Need more data.
                    match input.next().await {
                        Some(Ok(bytes)) => {
                            let s = String::from_utf8_lossy(&bytes);
                            buf.push_str(&s);
                            continue;
                        }
                        Some(Err(e)) => {
                            return Some((Err(e), (input, buf)));
                        }
                        None => {
                            // EOF: try parse remaining buffer once
                            if is_done(&buf) {
                                return None;
                            }
                            if let Some(v) = parse_payload(&buf) {
                                return Some((Ok(v), (input, String::new())));
                            }
                            return None;
                        }
                    }
                }
            }
        });

        Ok(Box::pin(stream))
    }
}

/// NDJSON / JSONL decoder (one JSON object per line).
pub struct NdjsonDecoder;

#[async_trait::async_trait]
impl Decoder for NdjsonDecoder {
    async fn decode_stream(
        &self,
        input: BoxStream<'static, Bytes>,
    ) -> PipeResult<BoxStream<'static, Value>> {
        let stream = stream::unfold(
            (input, String::new()),
            move |(mut input, mut buf)| async move {
                loop {
                    if let Some(idx) = buf.find('\n') {
                        let line = buf[..idx].trim().to_string();
                        buf = buf[idx + 1..].to_string();
                        if line.is_empty() {
                            continue;
                        }
                        match serde_json::from_str::<Value>(&line) {
                            Ok(v) => return Some((Ok(v), (input, buf))),
                            Err(e) => {
                                return Some((Err(crate::Error::Serialization(e)), (input, buf)))
                            }
                        }
                    }

                    match input.next().await {
                        Some(Ok(bytes)) => {
                            let s = String::from_utf8_lossy(&bytes);
                            buf.push_str(&s);
                            continue;
                        }
                        Some(Err(e)) => return Some((Err(e), (input, buf))),
                        None => {
                            let line = buf.trim();
                            if line.is_empty() {
                                return None;
                            }
                            match serde_json::from_str::<Value>(line) {
                                Ok(v) => return Some((Ok(v), (input, String::new()))),
                                Err(_) => return None,
                            }
                        }
                    }
                }
            },
        );

        Ok(Box::pin(stream))
    }
}

pub fn create_decoder(cfg: &DecoderConfig) -> Result<Box<dyn Decoder>, PipelineError> {
    match cfg.format.as_str() {
        "sse" => Ok(Box::new(SseDecoder::from_config(cfg)?)),
        // Many providers (e.g. Anthropic) still speak SSE but differ in event semantics.
        // We keep this manifest-driven and treat it as standard SSE framing.
        "anthropic_sse" => Ok(Box::new(SseDecoder::from_config(cfg)?)),
        "ndjson" | "jsonl" => Ok(Box::new(NdjsonDecoder)),
        other => Err(PipelineError::Configuration(format!(
            "Unsupported decoder format: {}. Supported formats: sse, jsonl, ndjson",
            other
        ))),
    }
}
