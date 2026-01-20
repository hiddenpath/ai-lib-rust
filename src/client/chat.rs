use crate::client::types::{cancel_pair, CancelHandle, ControlledStream};
use crate::types::{events::StreamingEvent, message::Message};
use crate::Result;
use futures::{stream::Stream, TryStreamExt};
use std::pin::Pin;

use super::core::{AiClient, UnifiedResponse};

/// Batch chat request parameters (developer-friendly, small surface).
#[derive(Debug, Clone)]
pub struct ChatBatchRequest {
    pub messages: Vec<Message>,
    pub temperature: Option<f64>,
    pub max_tokens: Option<u32>,
    pub tools: Option<Vec<crate::types::tool::ToolDefinition>>,
    pub tool_choice: Option<serde_json::Value>,
}

impl ChatBatchRequest {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            messages,
            temperature: None,
            max_tokens: None,
            tools: None,
            tool_choice: None,
        }
    }

    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
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

/// Builder for chat requests.
pub struct ChatRequestBuilder<'a> {
    pub(crate) client: &'a AiClient,
    pub(crate) messages: Vec<Message>,
    pub(crate) temperature: Option<f64>,
    pub(crate) max_tokens: Option<u32>,
    pub(crate) stream: bool,
    pub(crate) tools: Option<Vec<crate::types::tool::ToolDefinition>>,
    pub(crate) tool_choice: Option<serde_json::Value>,
}

impl<'a> ChatRequestBuilder<'a> {
    pub(crate) fn new(client: &'a AiClient) -> Self {
        Self {
            client,
            messages: Vec::new(),
            temperature: None,
            max_tokens: None,
            stream: false,
            tools: None,
            tool_choice: None,
        }
    }

    /// Add messages to the conversation.
    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }

    /// Set temperature.
    pub fn temperature(mut self, temp: f64) -> Self {
        self.temperature = Some(temp);
        self
    }

    /// Set max tokens.
    pub fn max_tokens(mut self, max: u32) -> Self {
        self.max_tokens = Some(max);
        self
    }

    /// Enable streaming.
    pub fn stream(mut self) -> Self {
        self.stream = true;
        self
    }

    /// Set tools for function calling.
    pub fn tools(mut self, tools: Vec<crate::types::tool::ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Set tool_choice (OpenAI-style).
    pub fn tool_choice(mut self, tool_choice: serde_json::Value) -> Self {
        self.tool_choice = Some(tool_choice);
        self
    }

    /// Execute the request and return a stream of events.
    pub async fn execute_stream(
        self,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamingEvent>> + Send + 'static>>> {
        let (stream, _cancel) = self.execute_stream_with_cancel().await?;
        Ok(stream)
    }

    /// Execute the request and return a cancellable stream of events plus per-call stats.
    ///
    /// Streaming semantics:
    /// - retry/fallback may happen only before any event is emitted to the caller
    /// - once an event is emitted, we will not retry automatically to avoid duplicate output
    pub async fn execute_stream_with_cancel_and_stats(
        self,
    ) -> Result<(
        Pin<Box<dyn Stream<Item = Result<StreamingEvent>> + Send + 'static>>,
        CancelHandle,
        crate::client::types::CallStats,
    )> {
        // Validate request against protocol capabilities
        self.client.validate_request(&self)?;

        let base_client = self.client;
        let unified_req = self.into_unified_request();

        // Pre-build fallback clients (async), then run unified policy loops.
        let mut fallback_clients: Vec<AiClient> = Vec::with_capacity(base_client.fallbacks.len());
        for model in &base_client.fallbacks {
            if let Ok(c) = base_client.with_model(model).await {
                fallback_clients.push(c);
            }
        }

        let (cancel_handle, cancel_rx) = cancel_pair();

        let mut last_err: Option<crate::Error> = None;

        for (candidate_idx, client) in std::iter::once(base_client)
            .chain(fallback_clients.iter())
            .enumerate()
        {
            let has_fallback = candidate_idx + 1 < (1 + fallback_clients.len());
            let policy = crate::client::policy::PolicyEngine::new(&client.manifest);
            let mut attempt: u32 = 0;
            let mut retry_count: u32 = 0;

            loop {
                // Pre-decision based on signals (skip known-bad candidates, e.g. breaker open).
                let sig = client.signals().await;
                if let Some(crate::client::policy::Decision::Fallback) =
                    policy.pre_decide(&sig, has_fallback)
                {
                    last_err = Some(crate::Error::runtime_with_context(
                        "skipped candidate due to signals",
                        crate::ErrorContext::new().with_source("policy_engine"),
                    ));
                    break;
                }

                let mut req = unified_req.clone();
                req.model = client.model_id.clone();

                match client.execute_stream_once(&req).await {
                    Ok((mut event_stream, permit, mut stats)) => {
                        // Peek the first item. If it errors BEFORE emitting anything, allow retry/fallback.
                        // If it yields an event, we commit to this stream (no more retry/fallback).
                        use futures::StreamExt;
                        let next_fut = event_stream.next();
                        let first = if let Some(t) = client.attempt_timeout {
                            match tokio::time::timeout(t, next_fut).await {
                                Ok(v) => v,
                                Err(_) => Some(Err(crate::Error::runtime_with_context(
                                    "attempt timeout",
                                    crate::ErrorContext::new().with_source("timeout_policy"),
                                ))),
                            }
                        } else {
                            next_fut.await
                        };

                        match first {
                            None => {
                                stats.retry_count = retry_count;
                                stats.emitted_any = false;
                                let wrapped = ControlledStream::new(
                                    Box::pin(futures::stream::empty()),
                                    Some(cancel_rx),
                                    permit,
                                );
                                return Ok((Box::pin(wrapped), cancel_handle, stats));
                            }
                            Some(Ok(first_ev)) => {
                                let first_ms = stats.duration_ms;
                                let stream = futures::stream::once(async move { Ok(first_ev) })
                                    .chain(event_stream);
                                let wrapped = ControlledStream::new(
                                    Box::pin(stream.map_err(|e| {
                                        // If it's already a crate::Error (like Transport error), preserve it.
                                        // Otherwise, it's likely a downstream pipeline error, wrap it.
                                        e
                                    })),
                                    Some(cancel_rx),
                                    permit,
                                );

                                stats.retry_count = retry_count;
                                stats.first_event_ms = Some(first_ms);
                                stats.emitted_any = true;

                                return Ok((Box::pin(wrapped), cancel_handle, stats));
                            }
                            Some(Err(e)) => {
                                let decision = policy.decide(&e, attempt, has_fallback)?;
                                last_err = Some(e);
                                match decision {
                                    crate::client::policy::Decision::Retry { delay } => {
                                        retry_count = retry_count.saturating_add(1);
                                        if delay.as_millis() > 0 {
                                            tokio::time::sleep(delay).await;
                                        }
                                        attempt = attempt.saturating_add(1);
                                        continue;
                                    }
                                    crate::client::policy::Decision::Fallback => break,
                                    crate::client::policy::Decision::Fail => {
                                        return Err(last_err.unwrap());
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let decision = policy.decide(&e, attempt, has_fallback)?;
                        last_err = Some(e);
                        match decision {
                            crate::client::policy::Decision::Retry { delay } => {
                                retry_count = retry_count.saturating_add(1);
                                if delay.as_millis() > 0 {
                                    tokio::time::sleep(delay).await;
                                }
                                attempt = attempt.saturating_add(1);
                                continue;
                            }
                            crate::client::policy::Decision::Fallback => break,
                            crate::client::policy::Decision::Fail => {
                                return Err(last_err.unwrap());
                            }
                        }
                    }
                }
            }
        }

        Err(last_err.unwrap_or_else(|| {
            crate::Error::runtime_with_context(
                "all streaming attempts failed",
                crate::ErrorContext::new().with_source("retry_policy"),
            )
        }))
    }

    /// Execute the request and return a cancellable stream of events.
    pub async fn execute_stream_with_cancel(
        self,
    ) -> Result<(
        Pin<Box<dyn Stream<Item = Result<StreamingEvent>> + Send + 'static>>,
        CancelHandle,
    )> {
        let (s, c, _stats) = self.execute_stream_with_cancel_and_stats().await?;
        Ok((s, c))
    }

    /// Execute the request and return the complete response.
    pub async fn execute(self) -> Result<UnifiedResponse> {
        let stream_flag = self.stream;
        let client = self.client;
        let unified_req = self.into_unified_request();

        // If streaming is not explicitly enabled, use non-streaming execution
        if !stream_flag {
            let (resp, _stats) = client.call_model_with_stats(unified_req).await?;
            return Ok(resp);
        }

        // For streaming requests, collect all events
        // Rebuild builder for streaming execution
        let mut stream = {
            let builder = ChatRequestBuilder {
                client,
                messages: unified_req.messages.clone(),
                temperature: unified_req.temperature,
                max_tokens: unified_req.max_tokens,
                stream: true,
                tools: unified_req.tools.clone(),
                tool_choice: unified_req.tool_choice.clone(),
            };
            builder.execute_stream().await?
        };
        let mut response = UnifiedResponse::default();
        let mut tool_asm = crate::utils::tool_call_assembler::ToolCallAssembler::new();

        use futures::StreamExt;
        let mut event_count = 0;
        while let Some(event) = stream.next().await {
            event_count += 1;
            match event? {
                StreamingEvent::PartialContentDelta { content, .. } => {
                    response.content.push_str(&content);
                }
                StreamingEvent::ToolCallStarted {
                    tool_call_id,
                    tool_name,
                    ..
                } => {
                    tool_asm.on_started(tool_call_id, tool_name);
                }
                StreamingEvent::PartialToolCall {
                    tool_call_id,
                    arguments,
                    ..
                } => {
                    tool_asm.on_partial(&tool_call_id, &arguments);
                }
                StreamingEvent::Metadata { usage, .. } => {
                    response.usage = usage;
                }
                StreamingEvent::StreamEnd { .. } => {
                    break;
                }
                other => {
                    // Log unexpected events for debugging
                    tracing::warn!("Unexpected event in execute(): {:?}", other);
                }
            }
        }

        if event_count == 0 {
            tracing::warn!(
                "No events received from stream. Possible causes: provider returned empty stream, \
                 network interruption, or event mapping configuration issue. Provider: {}, Model: {}",
                client.manifest.id,
                client.model_id
            );
        } else if response.content.is_empty() {
            tracing::warn!(
                "Received {} events but content is empty. This might indicate: (1) provider filtered \
                 content (safety/content policy), (2) non-streaming response format mismatch, \
                 (3) event mapping issue. Provider: {}, Model: {}",
                event_count,
                client.manifest.id,
                client.model_id
            );
        }

        response.tool_calls = tool_asm.finalize();

        Ok(response)
    }

    fn into_unified_request(self) -> crate::protocol::UnifiedRequest {
        crate::protocol::UnifiedRequest {
            operation: "chat".to_string(),
            model: self.client.model_id.clone(),
            messages: self.messages,
            temperature: self.temperature,
            max_tokens: self.max_tokens,
            stream: self.stream,
            tools: self.tools,
            tool_choice: self.tool_choice,
        }
    }
}
