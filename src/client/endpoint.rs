//! Endpoint resolution and service calls

use crate::protocol::{EndpointConfig, ProtocolError};
use crate::{Error, Result};
use std::future::Future;

use super::core::AiClient;

pub trait EndpointExt {
    fn resolve_endpoint(&self, name: &str) -> Result<&EndpointConfig>;

    /// Call a generic service by name. The returned future is `Send` and safe to use across threads.
    fn call_service(
        &self,
        service_name: &str,
    ) -> impl Future<Output = Result<serde_json::Value>> + Send;

    /// List models available from the provider. The returned future is `Send` and safe to use across threads.
    fn list_remote_models(&self) -> impl Future<Output = Result<Vec<String>>> + Send;
}

impl EndpointExt for AiClient {
    fn resolve_endpoint(&self, name: &str) -> Result<&EndpointConfig> {
        self.manifest
            .endpoints
            .as_ref()
            .and_then(|eps| eps.get(name))
            .ok_or_else(|| {
                Error::Protocol(ProtocolError::NotFound {
                    id: name.to_string(),
                    hint: None,
                })
            })
    }

    /// Call a generic service by name.
    async fn call_service(&self, service_name: &str) -> Result<serde_json::Value> {
        let service = self
            .manifest
            .services
            .as_ref()
            .and_then(|services| services.get(service_name))
            .ok_or_else(|| {
                Error::Protocol(ProtocolError::NotFound {
                    id: service_name.to_string(),
                    hint: None,
                })
            })?;

        self.transport
            .execute_service(
                &service.path,
                &service.method,
                service.headers.as_ref(),
                service.query_params.as_ref(),
            )
            .await
    }

    /// List models available from the provider.
    async fn list_remote_models(&self) -> Result<Vec<String>> {
        let response = self.call_service("list_models").await?;

        let models: Vec<String> = if let Some(data) = response.get("data") {
            data.as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| {
                    m.get("id")
                        .and_then(|id| id.as_str().map(|s| s.to_string()))
                })
                .collect()
        } else if let Some(models) = response.get("models") {
            // Gemini style
            models
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .filter_map(|m| {
                    m.get("name")
                        .and_then(|n| n.as_str().map(|s| s.to_string()))
                })
                .collect()
        } else {
            vec![]
        };

        Ok(models)
    }
}
