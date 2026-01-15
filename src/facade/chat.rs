use crate::client::chat::ChatRequestBuilder;
use crate::client::types::CallStats;
use crate::facade::provider::ModelRef;
use crate::types::message::Message;
use crate::client::core::UnifiedResponse;
use crate::{AiClient, CancelHandle, Result, StreamingEvent};
use futures::stream::Stream;
use std::pin::Pin;

/// Developer-friendly chat request facade.
///
/// This is intentionally minimal. It maps directly to the existing `ChatRequestBuilder`
/// and preserves manifest-first semantics.
#[derive(Debug, Clone)]
pub struct ChatCompletionRequest {
    pub model: Option<ModelRef>,
    pub messages: Vec<Message>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub stream: bool,
    pub tools: Option<Vec<crate::types::tool::ToolDefinition>>,
    pub tool_choice: Option<serde_json::Value>,
}

impl ChatCompletionRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            model: None,
            messages,
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: None,
            tool_choice: None,
        }
    }

    pub fn model(mut self, model: ModelRef) -> Self {
        self.model = Some(model);
        self
    }

    pub fn temperature(mut self, t: f64) -> Self {
        self.temperature = Some(t);
        self
    }

    pub fn max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = Some(n);
        self
    }

    pub fn stream(mut self) -> Self {
        self.stream = true;
        self
    }

    pub fn tools(mut self, tools: Vec<crate::types::tool::ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn tool_choice(mut self, tool_choice: serde_json::Value) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }
}

/// Facade methods on `AiClient`.
///
/// We avoid changing `AiClient` public surface for now; this keeps the facade opt-in.
pub trait ChatFacade {
    fn chat_completion_builder<'a>(&'a self, req: ChatCompletionRequest) -> Result<ChatRequestBuilder<'a>>;

    fn chat_completion<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<UnifiedResponse>> + Send + 'a>>;

    fn chat_completion_with_stats<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(UnifiedResponse, CallStats)>> + Send + 'a>>;

    fn chat_completion_stream<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Pin<Box<dyn Stream<Item = Result<StreamingEvent>> + Send + 'static>>>> + Send + 'a>>;

    fn chat_completion_stream_with_cancel<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(Pin<Box<dyn Stream<Item = Result<StreamingEvent>> + Send + 'static>>, CancelHandle)>> + Send + 'a>>;

    fn chat_completion_stream_with_cancel_and_stats<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(Pin<Box<dyn Stream<Item = Result<StreamingEvent>> + Send + 'static>>, CancelHandle, CallStats)>> + Send + 'a>>;
}

impl ChatFacade for AiClient {
    fn chat_completion_builder<'a>(&'a self, req: ChatCompletionRequest) -> Result<ChatRequestBuilder<'a>> {
        // If a model is explicitly provided, build a dedicated client (manifest-first) and return its builder.
        // For now we keep it simple: require the caller to build the model-specific client explicitly.
        //
        // This avoids surprising behavior where a single `AiClient` silently switches its model mid-flight.
        if req.model.is_some() {
            return Err(crate::Error::validation(
                "ChatCompletionRequest.model requires building a client for that model first. Use ModelRef::build_client() then call on that client.".to_string(),
            ));
        }

        let mut b = self.chat().messages(req.messages);
        if let Some(t) = req.temperature {
            b = b.temperature(t);
        }
        if let Some(m) = req.max_tokens {
            b = b.max_tokens(m);
        }
        if req.stream {
            b = b.stream();
        }
        if let Some(tools) = req.tools {
            b = b.tools(tools);
        }
        if let Some(tc) = req.tool_choice {
            b = b.tool_choice(tc);
        }
        Ok(b)
    }

    fn chat_completion<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<UnifiedResponse>> + Send + 'a>> {
        Box::pin(async move {
            let b = <AiClient as ChatFacade>::chat_completion_builder(self, req)?;
            b.execute().await
        })
    }

    fn chat_completion_with_stats<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(UnifiedResponse, CallStats)>> + Send + 'a>> {
        Box::pin(async move {
            // Keep facade minimal: we compile a UnifiedRequest and reuse the core stats path.
            // Note: we intentionally do NOT allow `req.model` to silently swap models on this client.
            if req.model.is_some() {
                return Err(crate::Error::validation(
                    "ChatCompletionRequest.model requires building a client for that model first. Use ModelRef::build_client() then call on that client.".to_string(),
                ));
            }

            let unified = crate::protocol::UnifiedRequest {
                operation: "chat".to_string(),
                model: self.model_id.clone(),
                messages: req.messages,
                temperature: req.temperature,
                max_tokens: req.max_tokens,
                stream: false,
                tools: req.tools,
                tool_choice: req.tool_choice,
            };

            self.call_model_with_stats(unified).await
        })
    }

    fn chat_completion_stream<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<Pin<Box<dyn Stream<Item = Result<StreamingEvent>> + Send + 'static>>>> + Send + 'a>> {
        Box::pin(async move {
            let b = <AiClient as ChatFacade>::chat_completion_builder(self, req)?;
            b.execute_stream().await
        })
    }

    fn chat_completion_stream_with_cancel<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(Pin<Box<dyn Stream<Item = Result<StreamingEvent>> + Send + 'static>>, CancelHandle)>> + Send + 'a>> {
        Box::pin(async move {
            let b = <AiClient as ChatFacade>::chat_completion_builder(self, req)?;
            b.execute_stream_with_cancel().await
        })
    }

    fn chat_completion_stream_with_cancel_and_stats<'a>(
        &'a self,
        req: ChatCompletionRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<(Pin<Box<dyn Stream<Item = Result<StreamingEvent>> + Send + 'static>>, CancelHandle, CallStats)>> + Send + 'a>> {
        Box::pin(async move {
            let mut b = <AiClient as ChatFacade>::chat_completion_builder(self, req)?;
            // Ensure streaming is enabled (stats are most meaningful for streaming here).
            b.stream = true;
            b.execute_stream_with_cancel_and_stats().await
        })
    }
}

