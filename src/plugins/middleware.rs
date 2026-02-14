//! Middleware system.

use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct MiddlewareContext {
    pub request: serde_json::Value,
    pub response: Option<serde_json::Value>,
    pub request_id: Option<String>,
    pub model: Option<String>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

impl MiddlewareContext {
    pub fn new(request: serde_json::Value) -> Self {
        Self {
            request,
            response: None,
            request_id: None,
            model: None,
            metadata: std::collections::HashMap::new(),
        }
    }
    pub fn set_response(&mut self, r: serde_json::Value) {
        self.response = Some(r);
    }
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
    pub fn with_model(mut self, m: impl Into<String>) -> Self {
        self.model = Some(m.into());
        self
    }
}

pub type NextFn<'a> = Box<
    dyn FnOnce(
            MiddlewareContext,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<MiddlewareContext>> + Send + 'a>,
        > + Send
        + 'a,
>;

#[async_trait]
pub trait Middleware: Send + Sync {
    async fn process(&self, ctx: MiddlewareContext, next: NextFn<'_>) -> Result<MiddlewareContext>;
    fn name(&self) -> &str {
        "unnamed"
    }
}

pub struct MiddlewareChain {
    middlewares: Vec<Arc<dyn Middleware>>,
}
impl MiddlewareChain {
    pub fn new() -> Self {
        Self {
            middlewares: Vec::new(),
        }
    }
    pub fn add(mut self, m: Arc<dyn Middleware>) -> Self {
        self.middlewares.push(m);
        self
    }
    pub fn len(&self) -> usize {
        self.middlewares.len()
    }
    pub fn is_empty(&self) -> bool {
        self.middlewares.is_empty()
    }

    pub async fn execute<F, Fut>(
        &self,
        ctx: MiddlewareContext,
        handler: F,
    ) -> Result<MiddlewareContext>
    where
        F: FnOnce(MiddlewareContext) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<MiddlewareContext>> + Send + 'static,
    {
        if self.middlewares.is_empty() {
            return handler(ctx).await;
        }
        let mut current = ctx;
        for mw in &self.middlewares {
            let next: NextFn<'_> = Box::new(move |c| Box::pin(async move { Ok(c) }));
            current = mw.process(current, next).await?;
        }
        handler(current).await
    }
}
impl Default for MiddlewareChain {
    fn default() -> Self {
        Self::new()
    }
}
