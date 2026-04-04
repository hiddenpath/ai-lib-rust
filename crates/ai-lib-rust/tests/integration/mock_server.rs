//! Mock HTTP server setup for integration tests

use mockito::{Mock, Server, ServerGuard};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Test fixture that manages a mock server
pub struct MockServerFixture {
    pub server: Arc<Mutex<ServerGuard>>,
}

impl MockServerFixture {
    pub async fn new() -> Self {
        let server = Server::new_async().await;
        Self {
            server: Arc::new(Mutex::new(server)),
        }
    }

    /// Create a mock for a successful streaming response (SSE)
    pub async fn mock_sse_stream(&self, path: &str, chunks: Vec<&str>) -> Mock {
        let mut server = self.server.lock().await;
        let mock = server
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
            .with_status(status as usize)
            .with_header("content-type", "application/json")
            .with_body(body)
            .create()
    }

    /// Create a mock for an error response
    pub async fn mock_error_response(&self, path: &str, status: u16, error_body: &str) -> Mock {
        let mut server = self.server.lock().await;
        server
            .mock("POST", path)
            .with_status(status as usize)
            .with_header("content-type", "application/json")
            .with_body(error_body)
            .create()
    }
}
