//! Mock HTTP server setup for integration tests

use ai_lib_rust::protocol::ProtocolManifest;
use ai_lib_rust::AiClient;
use mockito::{Mock, Server, ServerGuard};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test fixture that manages a mock server
pub struct MockServerFixture {
    pub server: Arc<Mutex<ServerGuard>>,
    pub base_url: String,
}

impl MockServerFixture {
    pub async fn new() -> Self {
        let server = Server::new_async().await;
        let base_url = server.url();
        Self {
            server: Arc::new(Mutex::new(server)),
            base_url,
        }
    }

    /// Create a test client with the mock server as base URL
    /// This uses the base_url_override feature to inject the mock server URL
    pub async fn create_test_client(&self, model: &str) -> ai_lib_rust::Result<AiClient> {
        // Use the builder's base_url_override to inject the mock server URL
        ai_lib_rust::AiClientBuilder::new()
            .base_url_override(&self.base_url)
            .build(model)
            .await
    }

    /// Create a mock for a successful streaming response (SSE)
    pub async fn mock_sse_stream(&self, path: &str, chunks: Vec<&str>) -> Mock {
        let mut server = self.server.lock().await;
        let mut mock = server
            .mock("POST", path)
            .with_status(200)
            .with_header("content-type", "text/event-stream");

        // Build SSE response body
        let body = chunks
            .iter()
            .map(|chunk| {
                if chunk.starts_with("data: ") {
                    format!("{}\n\n", chunk)
                } else {
                    format!("data: {}\n\n", chunk)
                }
            })
            .collect::<Vec<_>>()
            .join("");

        mock.with_body(&body).create()
    }

    /// Create a mock for a successful JSON response
    pub async fn mock_json_response(&self, path: &str, status: u16, body: &str) -> Mock {
        let mut server = self.server.lock().await;
        server
            .mock("POST", path)
            .with_status(status)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create()
    }

    /// Create a mock for an error response
    pub async fn mock_error_response(
        &self,
        path: &str,
        status: u16,
        error_body: &str,
    ) -> Mock {
        let mut server = self.server.lock().await;
        server
            .mock("POST", path)
            .with_status(status)
            .with_header("content-type", "application/json")
            .with_body(error_body)
            .create()
    }

    /// Create a mock that simulates network timeout (no response)
    pub async fn mock_timeout(&self, path: &str) -> Mock {
        let mut server = self.server.lock().await;
        // Mockito doesn't directly support timeouts, so we'll use a delay
        // For actual timeout testing, we'd need to configure the client timeout
        server
            .mock("POST", path)
            .with_status(200)
            .with_body("")
            .create()
    }
}
