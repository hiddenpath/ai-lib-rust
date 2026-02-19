//! Rerank client for document relevance scoring.

use super::types::{RerankOptions, RerankResult};
use crate::{Error, ErrorContext, Result};

/// Client for document reranking.
pub struct RerankerClient {
    http_client: reqwest::Client,
    model: String,
    base_url: String,
    endpoint_path: String,
    api_key: String,
}

impl RerankerClient {
    pub fn builder() -> RerankerClientBuilder {
        RerankerClientBuilder::new()
    }

    pub async fn rerank(
        &self,
        query: &str,
        documents: &[impl AsRef<str>],
        options: &RerankOptions,
    ) -> Result<Vec<RerankResult>> {
        let endpoint = format!("{}{}", self.base_url.trim_end_matches('/'), self.endpoint_path);
        let docs: Vec<String> = documents.iter().map(|d| d.as_ref().to_string()).collect();
        let mut body = serde_json::json!({
            "model": self.model,
            "query": query,
            "documents": docs,
        });
        if let Some(top_n) = options.top_n {
            body["top_n"] = serde_json::json!(top_n);
        }
        if let Some(max_tokens) = options.max_tokens_per_doc {
            body["max_tokens_per_doc"] = serde_json::json!(max_tokens);
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
                    format!("Rerank request failed: {}", e),
                    ErrorContext::new().with_source("rerank"),
                )
            })?;
        let status = response.status();
        let body_str = response.text().await.map_err(|e| {
            Error::network_with_context(
                format!("Failed to read Rerank response: {}", e),
                ErrorContext::new(),
            )
        })?;
        if !status.is_success() {
            return Err(Error::api_with_context(
                format!("Rerank API error ({}): {}", status, body_str),
                ErrorContext::new(),
            ));
        }
        let json: serde_json::Value = serde_json::from_str(&body_str)?;
        let results = json
            .get("results")
            .and_then(|v| v.as_array())
            .ok_or_else(|| Error::api_with_context("Invalid rerank response: missing results", ErrorContext::new()))?;
        let mut out = Vec::with_capacity(results.len());
        for r in results {
            let index = r.get("index").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let relevance_score = r
                .get("relevance_score")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as f32;
            let document = r.get("document").and_then(|v| v.as_str()).map(String::from);
            out.push(RerankResult {
                index,
                relevance_score,
                document,
            });
        }
        Ok(out)
    }

    pub fn model(&self) -> &str {
        &self.model
    }
}

pub struct RerankerClientBuilder {
    model: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    endpoint_path: Option<String>,
    timeout_secs: u64,
}

impl RerankerClientBuilder {
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

    pub async fn build(self) -> Result<RerankerClient> {
        let model = self
            .model
            .ok_or_else(|| Error::configuration("Model must be specified"))?;
        let api_key = self
            .api_key
            .or_else(|| std::env::var("COHERE_API_KEY").ok())
            .ok_or_else(|| Error::configuration("API key required (COHERE_API_KEY)"))?;
        let base_url = self
            .base_url
            .unwrap_or_else(|| "https://api.cohere.com/v2".to_string());
        let endpoint_path = self
            .endpoint_path
            .unwrap_or_else(|| "/rerank".to_string());
        let endpoint_path = if endpoint_path.starts_with('/') {
            endpoint_path
        } else {
            format!("/{}", endpoint_path)
        };
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(self.timeout_secs))
            .build()
            .map_err(|e| Error::configuration(format!("Failed to create HTTP client: {}", e)))?;
        Ok(RerankerClient {
            http_client,
            model,
            base_url,
            endpoint_path,
            api_key,
        })
    }
}

impl Default for RerankerClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
