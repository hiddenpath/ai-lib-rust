//! Embedding client for generating embeddings.

use super::types::{Embedding, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage};
use crate::{Error, ErrorContext, Result};

pub struct EmbeddingClient {
    http_client: reqwest::Client,
    model: String,
    base_url: String,
    api_key: String,
    dimensions: Option<usize>,
    max_batch_size: usize,
}

impl EmbeddingClient {
    pub fn builder() -> EmbeddingClientBuilder {
        EmbeddingClientBuilder::new()
    }

    pub async fn embed(&self, text: &str) -> Result<EmbeddingResponse> {
        let request = EmbeddingRequest::single(&self.model, text);
        self.execute(request).await
    }

    pub async fn embed_batch(&self, texts: &[impl AsRef<str>]) -> Result<EmbeddingResponse> {
        let texts: Vec<String> = texts.iter().map(|t| t.as_ref().to_string()).collect();
        if texts.len() <= self.max_batch_size {
            return self
                .execute(EmbeddingRequest::batch(&self.model, texts))
                .await;
        }
        let mut all_embeddings: Vec<Embedding> = Vec::new();
        let mut total_usage = EmbeddingUsage::default();
        for (batch_idx, chunk) in texts.chunks(self.max_batch_size).enumerate() {
            let response = self
                .execute(EmbeddingRequest::batch(&self.model, chunk.to_vec()))
                .await?;
            let offset = batch_idx * self.max_batch_size;
            for mut emb in response.embeddings {
                emb.index += offset;
                all_embeddings.push(emb);
            }
            total_usage.add(&response.usage);
        }
        Ok(EmbeddingResponse::new(
            all_embeddings,
            self.model.clone(),
            total_usage,
        ))
    }

    async fn execute(&self, mut request: EmbeddingRequest) -> Result<EmbeddingResponse> {
        if let Some(dims) = self.dimensions {
            request = request.with_dimensions(dims);
        }
        let endpoint = format!("{}/v1/embeddings", self.base_url);
        let response = self
            .http_client
            .post(&endpoint)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                Error::network_with_context(
                    format!("Embedding request failed: {}", e),
                    ErrorContext::new().with_source("embeddings"),
                )
            })?;
        let status = response.status();
        let body = response.text().await.map_err(|e| {
            Error::network_with_context(
                format!("Failed to read response: {}", e),
                ErrorContext::new(),
            )
        })?;
        if !status.is_success() {
            return Err(Error::api_with_context(
                format!("Embedding API error ({}): {}", status, body),
                ErrorContext::new(),
            ));
        }
        let json: serde_json::Value = serde_json::from_str(&body)?;
        EmbeddingResponse::from_openai_format(&json)
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

pub struct EmbeddingClientBuilder {
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    dimensions: Option<usize>,
    max_batch_size: usize,
    timeout_secs: u64,
}

impl EmbeddingClientBuilder {
    pub fn new() -> Self {
        Self {
            model: None,
            api_key: None,
            base_url: None,
            dimensions: None,
            max_batch_size: 100,
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
    pub fn dimensions(mut self, dimensions: usize) -> Self {
        self.dimensions = Some(dimensions);
        self
    }

    pub async fn build(self) -> Result<EmbeddingClient> {
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
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| Error::configuration(format!("Failed to create HTTP client: {}", e)))?;
        Ok(EmbeddingClient {
            http_client,
            model,
            base_url,
            api_key,
            dimensions: self.dimensions,
            max_batch_size: self.max_batch_size,
        })
    }
}

impl Default for EmbeddingClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
