//! 请求执行逻辑：单次尝试的流式与非流式请求执行。
//!
//! Request execution logic (single-attempt).

use crate::client::types::CallStats;
use crate::types::events::StreamingEvent;
use crate::{Error, Result};
use futures::{StreamExt, TryStreamExt};
use std::pin::Pin;
use tracing::info;
use uuid::Uuid;

use super::core::{AiClient, UnifiedResponse};
use super::endpoint::EndpointExt;
use super::error_classification::is_fallbackable_error_class;
use super::preflight::PreflightExt;

impl AiClient {
    fn error_code_from_body(&self, body: &str) -> Option<String> {
        let json: serde_json::Value = serde_json::from_str(body).ok()?;

        // Prefer protocol-driven mappings if present
        if let Some(features) = &self.manifest.features {
            if let Some(rm) = &features.response_mapping {
                if let Some(em) = &rm.error {
                    if let Some(code_path) = &em.code_path {
                        if let Some(v) =
                            crate::utils::json_path::PathMapper::get_string(&json, code_path)
                        {
                            return Some(v);
                        }
                    }
                }
            }
        }

        // Fallback to the common OpenAI-style error shape
        json.get("error")
            .and_then(|e| e.get("code"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    }

    fn is_model_routing_error(status: u16, code: Option<&str>, body: &str) -> bool {
        // Conservative gating: only treat some 4xx as "try another model/provider".
        if status != 400 && status != 404 {
            return false;
        }

        if let Some(code) = code {
            matches!(
                code,
                "model_decommissioned"
                    | "model_not_found"
                    | "model_not_supported"
                    | "invalid_model"
            )
        } else {
            // Heuristic fallback for providers that don't expose a structured code.
            let b = body.to_lowercase();
            b.contains("model")
                && (b.contains("decommission")
                    || b.contains("not found")
                    || b.contains("no longer supported"))
        }
    }

    fn is_transient_server_status(status: u16) -> bool {
        status >= 500 && status <= 599
    }
    /// Start a streaming request and return the event stream.
    ///
    /// This is a single attempt (no retry/fallback). Higher-level policy loops live in the caller.
    pub(crate) async fn execute_stream_once<'a>(
        &self,
        request: &crate::protocol::UnifiedRequest,
    ) -> Result<(
        Pin<Box<dyn futures::stream::Stream<Item = Result<StreamingEvent>> + Send + 'static>>,
        Option<tokio::sync::OwnedSemaphorePermit>,
        CallStats,
    )> {
        let permit = PreflightExt::preflight(self).await?;
        let client_request_id = Uuid::new_v4().to_string();

        let provider_request = self.manifest.compile_request(request)?;
        let endpoint = EndpointExt::resolve_endpoint(self, &request.operation)?;

        let start = std::time::Instant::now();
        let resp = self
            .transport
            .execute_stream_response(
                &endpoint.method,
                &endpoint.path,
                &provider_request,
                Some(&client_request_id),
            )
            .await?;

        // Extract rate limits immediately from any response (success or error)
        PreflightExt::update_rate_limits(self, resp.headers()).await;

        if !resp.status().is_success() {
            PreflightExt::on_failure(self);
            let status = resp.status().as_u16();
            let class = self
                .manifest
                .error_classification
                .as_ref()
                .and_then(|ec| ec.by_http_status.as_ref())
                .and_then(|m: &std::collections::HashMap<String, String>| m.get(&status.to_string()).cloned())
                .unwrap_or_else(|| "http_error".to_string());

            // Protocol-driven fallback decision: use standard error_classes guidance
            // from spec.yaml. Transient errors (retryable) are typically fallbackable.
            let mut should_fallback = is_fallbackable_error_class(class.as_str());

            let headers = resp.headers().clone();
            let retry_after_ms = PreflightExt::retry_after_ms(self, &headers);
            let body = resp.text().await.unwrap_or_default();

            // Extract provider error code once and reuse
            let provider_code = self.error_code_from_body(&body);
            if !should_fallback {
                should_fallback = Self::is_model_routing_error(status, provider_code.as_deref(), &body);
            }

            let retry_policy = self.manifest.retry_policy.as_ref();
            let retryable = retry_policy
                .and_then(|p| p.retry_on_http_status.as_ref())
                .map(|v: &Vec<u16>| v.contains(&status))
                .unwrap_or(false);

            // Derive V2 standard error code for structured classification
            let std_code = provider_code
                .as_deref()
                .and_then(crate::error_code::StandardErrorCode::from_provider_code)
                .unwrap_or_else(|| crate::error_code::StandardErrorCode::from_http_status(status));

            info!(
                http_status = status,
                error_class = class.as_str(),
                standard_code = std_code.code(),
                endpoint = endpoint.path.as_str(),
                duration_ms = start.elapsed().as_millis(),
                "ai-lib-rust streaming request failed"
            );

            let upstream = PreflightExt::header_first(
                self,
                &headers,
                &["x-request-id", "request-id", "x-amzn-requestid", "cf-ray"],
            );
            let mut context = crate::ErrorContext::new()
                .with_status_code(status)
                .with_request_id(client_request_id.clone())
                .with_retryable(retryable)
                .with_fallbackable(should_fallback)
                .with_standard_code(std_code)
                .with_source("execute_stream_once");
            if let Some(ref ec) = provider_code {
                context = context.with_error_code(ec.clone());
            }
            if let Some(up) = upstream {
                context = context.with_details(format!("upstream_id: {}", up));
            }

            return Err(Error::Remote {
                status,
                class,
                message: body,
                retryable,
                fallbackable: should_fallback,
                retry_after_ms,
                context: None,
            }
            .with_context(context));
        }

        PreflightExt::on_success(self);

        let upstream_request_id = PreflightExt::header_first(
            self,
            resp.headers(),
            &["x-request-id", "request-id", "x-amzn-requestid", "cf-ray"],
        );
        let http_status = resp.status().as_u16();

        let response_stream: crate::BoxStream<'static, bytes::Bytes> = Box::pin(
            resp.bytes_stream()
                .map_err(|e| Error::Transport(crate::transport::TransportError::Http(e))),
        );
        let event_stream = self
            .pipeline
            .clone()
            .process_stream_arc(response_stream)
            .await?;

        let stats = CallStats {
            model: request.model.clone(),
            operation: request.operation.clone(),
            endpoint: endpoint.path.clone(),
            http_status,
            retry_count: 0,
            duration_ms: start.elapsed().as_millis(),
            first_event_ms: None,
            emitted_any: false,
            client_request_id,
            upstream_request_id,
            error_class: None,
            usage: None,
            signals: self.signals().await,
        };

        Ok((event_stream, permit, stats))
    }

    pub(crate) async fn execute_once_with_stats(
        &self,
        request: &crate::protocol::UnifiedRequest,
    ) -> Result<(UnifiedResponse, CallStats)> {
        let _permit = self.preflight().await?;

        let client_request_id = Uuid::new_v4().to_string();

        // Compile unified request to provider-specific format
        let provider_request = self.manifest.compile_request(request)?;

        // Resolve endpoint based on request intent (operation)
        let endpoint = self.resolve_endpoint(&request.operation)?;

        let start = std::time::Instant::now();

        let mut last_upstream_request_id: Option<String> = None;
        let resp = self
            .transport
            .execute_stream_response(
                &endpoint.method,
                &endpoint.path,
                &provider_request,
                Some(&client_request_id),
            )
            .await?;

        // Extract rate limits immediately
        self.update_rate_limits(resp.headers()).await;

        // For non-streaming requests, handle as complete JSON response
        if !request.stream {
            let status = resp.status().as_u16();
            let headers = resp.headers().clone(); // Clone headers before consuming resp

            // Status-based error classification
            if !resp.status().is_success() {
                PreflightExt::on_failure(self);
                let class = self
                    .manifest
                    .error_classification
                    .as_ref()
                    .and_then(|ec| ec.by_http_status.as_ref())
                    .and_then(|m: &std::collections::HashMap<String, String>| m.get(&status.to_string()).cloned())
                    .unwrap_or_else(|| "http_error".to_string());

                let should_fallback = is_fallbackable_error_class(class.as_str())
                    || Self::is_transient_server_status(status);
                let body = resp.text().await.unwrap_or_default();
                let retry_policy = self.manifest.retry_policy.as_ref();
                let retryable = retry_policy
                    .and_then(|p| p.retry_on_http_status.as_ref())
                    .map(|v: &Vec<u16>| v.contains(&status))
                    .unwrap_or(false);
                let retry_after_ms = PreflightExt::retry_after_ms(self, &headers);

                // Extract provider error code once and derive standard code
                let provider_code = self.error_code_from_body(&body);
                let std_code = provider_code
                    .as_deref()
                    .and_then(crate::error_code::StandardErrorCode::from_provider_code)
                    .unwrap_or_else(|| crate::error_code::StandardErrorCode::from_http_status(status));

                let mut context = crate::ErrorContext::new()
                    .with_status_code(status)
                    .with_request_id(client_request_id)
                    .with_retryable(retryable)
                    .with_fallbackable(should_fallback)
                    .with_standard_code(std_code)
                    .with_source("execution_once");

                if let Some(upstream_id) = PreflightExt::header_first(
                    self,
                    &headers,
                    &["x-request-id", "request-id", "x-amzn-requestid", "cf-ray"],
                ) {
                    context = context.with_details(format!("upstream_id: {}", upstream_id));
                }
                if let Some(ref ec) = provider_code {
                    context = context.with_error_code(ec.clone());
                }

                return Err(Error::Remote {
                    status,
                    class,
                    message: body,
                    retryable,
                    fallbackable: should_fallback,
                    retry_after_ms,
                    context: None,
                }
                .with_context(context));
            }

            // Read the entire response body
            let body_bytes = resp
                .bytes()
                .await
                .map_err(|e| Error::Transport(crate::transport::TransportError::Http(e)))?;
            let body_text = String::from_utf8_lossy(&body_bytes);

            // Parse as JSON and extract using response_paths
            let json: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
                Error::runtime_with_context(
                    format!("Failed to parse response JSON: {}", e),
                    crate::ErrorContext::new().with_source("json_parse"),
                )
            })?;

            let mut response = UnifiedResponse::default();

            // Extract content using response_paths
            if let Some(paths) = &self.manifest.response_paths {
                let paths: &std::collections::HashMap<String, String> = paths;
                if let Some(content_path) = paths.get("content") {
                    if let Some(content) =
                        crate::utils::json_path::PathMapper::get_string(&json, content_path)
                    {
                        response.content = content;
                    }
                }
                if let Some(usage_path) = paths.get("usage") {
                    if let Some(usage_value) =
                        crate::utils::json_path::PathMapper::get_path(&json, usage_path)
                    {
                        response.usage = Some(usage_value.clone());
                    }
                }
                // TODO: Extract tool_calls if needed
            }

            if last_upstream_request_id.is_none() {
                last_upstream_request_id = PreflightExt::header_first(
                    self,
                    &headers,
                    &["x-request-id", "request-id", "x-amzn-requestid", "cf-ray"],
                );
            }

            let stats = CallStats {
                model: request.model.clone(),
                operation: request.operation.clone(),
                endpoint: endpoint.path.clone(),
                http_status: status,
                retry_count: 0,
                duration_ms: start.elapsed().as_millis(),
                first_event_ms: None,
                emitted_any: true,
                client_request_id,
                upstream_request_id: last_upstream_request_id,
                error_class: None,
                usage: response.usage.clone(),
                signals: self.signals().await,
            };

            self.on_success();
            return Ok((response, stats));
        }

        // Status-based error classification (protocol-driven) + fallback decision
        if !resp.status().is_success() {
            PreflightExt::on_failure(self);
            let status = resp.status().as_u16();
            let class = self
                .manifest
                .error_classification
                .as_ref()
                .and_then(|ec| ec.by_http_status.as_ref())
                .and_then(|m: &std::collections::HashMap<String, String>| m.get(&status.to_string()).cloned())
                .unwrap_or_else(|| "http_error".to_string());

            // Protocol-driven fallback decision: use standard error_classes guidance
            // from spec.yaml. Transient errors (retryable) are typically fallbackable.
            let mut should_fallback = is_fallbackable_error_class(class.as_str());

            let headers = resp.headers().clone();
            let request_id = PreflightExt::header_first(
                self,
                &headers,
                &["x-request-id", "request-id", "x-amzn-requestid", "cf-ray"],
            );
            let body = resp.text().await.unwrap_or_default();

            // Extract provider error code once and reuse
            let provider_code = self.error_code_from_body(&body);
            if !should_fallback {
                should_fallback = Self::is_model_routing_error(status, provider_code.as_deref(), &body);
            }
            if !should_fallback && Self::is_transient_server_status(status) {
                should_fallback = true;
            }

            let retry_policy = self.manifest.retry_policy.as_ref();
            let retryable = retry_policy
                .and_then(|p| p.retry_on_http_status.as_ref())
                .map(|v: &Vec<u16>| v.contains(&status))
                .unwrap_or(false);
            let retry_after_ms = PreflightExt::retry_after_ms(self, &headers);

            // Derive V2 standard error code
            let std_code = provider_code
                .as_deref()
                .and_then(crate::error_code::StandardErrorCode::from_provider_code)
                .unwrap_or_else(|| crate::error_code::StandardErrorCode::from_http_status(status));

            info!(
                http_status = status,
                error_class = class.as_str(),
                standard_code = std_code.code(),
                request_id = request_id.as_deref().unwrap_or(""),
                endpoint = endpoint.path.as_str(),
                duration_ms = start.elapsed().as_millis(),
                "ai-lib-rust request failed"
            );

            let mut context = crate::ErrorContext::new()
                .with_status_code(status)
                .with_request_id(client_request_id.clone())
                .with_retryable(retryable)
                .with_fallbackable(should_fallback)
                .with_standard_code(std_code)
                .with_source("execute_once_streaming");
            if let Some(ref ec) = provider_code {
                context = context.with_error_code(ec.clone());
            }
            if let Some(up) = request_id {
                context = context.with_details(format!("upstream_id: {}", up));
            }

            return Err(Error::Remote {
                status,
                class,
                message: body,
                retryable,
                fallbackable: should_fallback,
                retry_after_ms,
                context: None,
            }
            .with_context(context));
        }

        info!(
            http_status = resp.status().as_u16(),
            client_request_id = client_request_id.as_str(),
            endpoint = endpoint.path.as_str(),
            duration_ms = start.elapsed().as_millis(),
            "ai-lib-rust request started streaming"
        );
        self.on_success();

        if last_upstream_request_id.is_none() {
            last_upstream_request_id = PreflightExt::header_first(
                self,
                resp.headers(),
                &["x-request-id", "request-id", "x-amzn-requestid", "cf-ray"],
            );
        }

        // For streaming requests, use pipeline
        let http_status = resp.status().as_u16();
        let response_stream: crate::BoxStream<'static, bytes::Bytes> = Box::pin(
            resp.bytes_stream()
                .map_err(|e| Error::Transport(crate::transport::TransportError::Http(e))),
        );
        let mut event_stream = self
            .pipeline
            .clone()
            .process_stream_arc(response_stream)
            .await?;

        let mut response = UnifiedResponse::default();
        let mut tool_asm = crate::utils::tool_call_assembler::ToolCallAssembler::new();

        while let Some(event) = event_stream.next().await {
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
                _ => {}
            }
        }

        response.tool_calls = tool_asm.finalize();

        let stats = CallStats {
            model: request.model.clone(),
            operation: request.operation.clone(),
            endpoint: endpoint.path.clone(),
            http_status,
            retry_count: 0,
            duration_ms: start.elapsed().as_millis(),
            first_event_ms: None,
            emitted_any: true,
            client_request_id,
            upstream_request_id: last_upstream_request_id,
            error_class: None,
            usage: response.usage.clone(),
            signals: self.signals().await,
        };

        Ok((response, stats))
    }
}
