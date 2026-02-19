//! TTS (Text-to-Speech) client.

use super::types;
use super::types::{AudioOutput, TtsOptions};
use crate::{Error, ErrorContext, Result};

/// Client for text-to-speech synthesis.
pub struct TtsClient {
    http_client: reqwest::Client,
    model: String,
    base_url: String,
    endpoint_path: String,
    api_key: String,
}

impl TtsClient {
    pub fn builder() -> TtsClientBuilder {
        TtsClientBuilder::new()
    }

    pub async fn synthesize(&self, text: &str, options: &TtsOptions) -> Result<AudioOutput> {
        let endpoint = format!("{}{}", self.base_url.trim_end_matches('/'), self.endpoint_path);
        let mut body = serde_json::json!({
            "model": self.model,
            "input": text,
        });
        if let Some(voice) = &options.voice {
            body["voice"] = serde_json::Value::String(voice.clone());
        }
        if let Some(speed) = options.speed {
            body["speed"] = serde_json::json!(speed);
        }
        if let Some(rf) = &options.response_format {
            body["response_format"] = serde_json::Value::String(rf.clone());
        }
        let response = self
            .http_client
            .post(&endpoint)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| {
                Error::network_with_context(
                    format!("TTS request failed: {}", e),
                    ErrorContext::new().with_source("tts"),
                )
            })?;
        let status = response.status();
        let bytes = response.bytes().await.map_err(|e| {
            Error::network_with_context(
                format!("Failed to read TTS response: {}", e),
                ErrorContext::new(),
            )
        })?;
        if !status.is_success() {
            let body_str = String::from_utf8_lossy(&bytes);
            return Err(Error::api_with_context(
                format!("TTS API error ({}): {}", status, body_str),
                ErrorContext::new(),
            ));
        }
        let format = options
            .response_format
            .as_deref()
            .map(types::AudioFormat::from_str)
            .unwrap_or(types::AudioFormat::Mp3);
        Ok(AudioOutput {
            data: bytes.to_vec(),
            format,
        })
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

pub struct TtsClientBuilder {
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    endpoint_path: Option<String>,
    timeout_secs: u64,
}

impl TtsClientBuilder {
    pub fn new() -> Self {
        Self {
            model: None,
            api_key: None,
            base_url: None,
            endpoint_path: None,
            timeout_secs: 60,
        }
    }
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = Some(url.into());
        self
    }
    pub fn endpoint_path(mut self, path: impl Into<String>) -> Self {
        self.endpoint_path = Some(path.into());
        self
    }

    pub async fn build(self) -> Result<TtsClient> {
        let model = self
            .model
            .ok_or_else(|| Error::configuration("Model must be specified"))?;
        let api_key = self
            .api_key
            .or_else(|| std::env::var("OPENAI_API_KEY").ok())
            .ok_or_else(|| Error::configuration("API key required"))?;
        let base_url = self
            .base_url
            .unwrap_or_else(|| "https://api.openai.com".to_string());
        let endpoint_path = self
            .endpoint_path
            .unwrap_or_else(|| "/v1/audio/speech".to_string());
        let endpoint_path = if endpoint_path.starts_with('/') {
            endpoint_path
        } else {
            format!("/{}", endpoint_path)
        };
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| Error::configuration(format!("Failed to create HTTP client: {}", e)))?;
        Ok(TtsClient {
            http_client,
            model,
            base_url,
            endpoint_path,
            api_key,
        })
    }
}

impl Default for TtsClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
