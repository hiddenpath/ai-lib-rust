use crate::protocol::ProtocolManifest;
use crate::{BoxStream, Result};
use bytes::Bytes;
use futures::TryStreamExt;
use keyring::Entry;
use std::env;
use std::time::Duration;
use reqwest::Proxy;

pub struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

impl HttpTransport {
    pub fn new(manifest: &ProtocolManifest, model: &str) -> Result<Self> {
        let provider_id = manifest.provider_id.as_deref().unwrap_or(&manifest.id);
        let api_key = Self::get_api_key(provider_id);

        // Minimal production-friendly defaults (env-overridable).
        let timeout_secs = env::var("AI_HTTP_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .or_else(|| env::var("AI_TIMEOUT_SECS").ok().and_then(|s| s.parse::<u64>().ok()))
            .unwrap_or(30);

        let mut builder = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .pool_max_idle_per_host(
                env::var("AI_HTTP_POOL_MAX_IDLE_PER_HOST")
                    .ok()
                    .and_then(|s| s.parse::<usize>().ok())
                    .unwrap_or(32),
            )
            .pool_idle_timeout(Some(Duration::from_secs(
                env::var("AI_HTTP_POOL_IDLE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(90),
            )))
            // Conservative HTTP/2 keepalive defaults for long-lived connections.
            // (No extra env knobs for now to keep developer UI simple.)
            .http2_adaptive_window(true)
            .http2_keep_alive_interval(Some(Duration::from_secs(30)))
            .http2_keep_alive_timeout(Duration::from_secs(10));

        if let Ok(proxy_url) = env::var("AI_PROXY_URL") {
            if let Ok(proxy) = Proxy::all(&proxy_url) {
                builder = builder.proxy(proxy);
            }
        }

        let client = builder
            .build()
            .map_err(|e| crate::Error::Transport(crate::transport::TransportError::Other(e.to_string())))?;

        Ok(Self {
            client,
            base_url: manifest.base_url.clone(),
            model: model.to_string(),
            api_key,
        })
    }

    fn get_api_key(provider_id: &str) -> Option<String> {
        // 1. Try Keyring
        let entry = Entry::new("ai-protocol", provider_id).ok();
        if let Some(entry) = entry {
            if let Ok(key) = entry.get_password() {
                return Some(key);
            }
        }

        // 2. Try Environment Variable (PROVIDER_API_KEY)
        let env_var = format!("{}_API_KEY", provider_id.to_uppercase());
        env::var(env_var).ok()
    }

    pub async fn execute_stream_response(
        &self,
        method: &str,
        path: &str,
        request_body: &serde_json::Value,
        client_request_id: Option<&str>,
    ) -> Result<reqwest::Response> {
        let interpolated_path = path.replace("{model}", &self.model);
        let url = format!("{}{}", self.base_url, interpolated_path);

        let mut req = match method.to_uppercase().as_str() {
            "POST" => self.client.post(&url).json(request_body),
            "PUT" => self.client.put(&url).json(request_body),
            "DELETE" => self.client.delete(&url),
            _ => self.client.get(&url),
        };

        if let Some(key) = &self.api_key {
            req = req.bearer_auth(key);
        }

        // Prefer SSE for providers that support it
        req = req.header("accept", "text/event-stream");
        if let Some(id) = client_request_id {
            // Our own correlation id. Providers may ignore it, but applications can use it for linkage.
            req = req.header("x-ai-protocol-request-id", id);
        }

        req.send()
            .await
            .map_err(|e| crate::Error::Transport(crate::transport::TransportError::Http(e)))
    }

    pub async fn execute_stream<'a>(
        &'a self,
        method: &str,
        path: &str,
        request_body: &serde_json::Value,
    ) -> Result<BoxStream<'a, Bytes>> {
        let resp = self
            .execute_stream_response(method, path, request_body, None)
            .await?;

        // Convert reqwest bytes stream to our unified BoxStream
        let byte_stream = resp
            .bytes_stream()
            .map_err(|e| crate::Error::Transport(crate::transport::TransportError::Http(e)));
        Ok(Box::pin(byte_stream))
    }

    pub async fn execute_get(&self, path: &str) -> Result<serde_json::Value> {
        self.execute_service(path, "GET", None, None).await
    }

    pub async fn execute_service(
        &self,
        path: &str,
        method: &str,
        headers: Option<&std::collections::HashMap<String, String>>,
        query_params: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<serde_json::Value> {
        let interpolated_path = path.replace("{model}", &self.model);
        let url = format!("{}{}", self.base_url, interpolated_path);
        let mut request = match method.to_uppercase().as_str() {
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => self.client.get(&url),
        };

        if let Some(key) = &self.api_key {
            request = request.bearer_auth(key);
        }

        if let Some(headers) = headers {
            for (k, v) in headers {
                request = request.header(k, v);
            }
        }

        if let Some(params) = query_params {
            request = request.query(params);
        }

        let response = request
            .send()
            .await
            .map_err(|e| crate::Error::Transport(crate::transport::TransportError::Http(e)))?;

        let json = response
            .json()
            .await
            .map_err(|e| crate::Error::Transport(crate::transport::TransportError::Http(e)))?;

        Ok(json)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Transport error: {0}")]
    Other(String),
}
