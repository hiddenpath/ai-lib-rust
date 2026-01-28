//! Interceptor pipeline example (application-layer hooks).
//!
//! Run:
//! - `cargo run --example interceptors --features interceptors`

#[cfg(feature = "interceptors")]
use ai_lib_rust::interceptors::{Interceptor, InterceptorPipeline, RequestContext};

#[cfg(feature = "interceptors")]
use ai_lib_rust::{AiClient, Result};

#[cfg(feature = "interceptors")]
use async_trait::async_trait;

#[cfg(feature = "interceptors")]
struct Logger;

#[cfg(feature = "interceptors")]
#[async_trait]
impl Interceptor for Logger {
    async fn on_request(&self, ctx: &RequestContext, req: &ai_lib_rust::protocol::UnifiedRequest) {
        println!(
            "📡 [Request] provider={} model={} op={} messages={}",
            ctx.provider,
            ctx.model,
            ctx.operation,
            req.messages.len()
        );
    }

    async fn on_response(
        &self,
        ctx: &RequestContext,
        _req: &ai_lib_rust::protocol::UnifiedRequest,
        _resp: &ai_lib_rust::client::UnifiedResponse,
    ) {
        println!(
            "✅ [Response] provider={} model={} op={}",
            ctx.provider, ctx.model, ctx.operation
        );
    }

    async fn on_error(
        &self,
        ctx: &RequestContext,
        _req: &ai_lib_rust::protocol::UnifiedRequest,
        err: &ai_lib_rust::Error,
    ) {
        eprintln!(
            "❌ [Error] provider={} model={} op={} err={}",
            ctx.provider, ctx.model, ctx.operation, err
        );
    }
}

#[cfg(feature = "interceptors")]
#[tokio::main]
async fn main() -> Result<()> {
    // NOTE: This example loads a manifest; set AI_PROTOCOL_PATH for offline runs if needed.
    let client = AiClient::new("deepseek/deepseek-chat").await?;

    let pipeline = InterceptorPipeline::new().with(Logger);
    let ctx = RequestContext {
        provider: client
            .manifest
            .provider_id
            .clone()
            .unwrap_or_else(|| client.manifest.id.clone()),
        model: client.manifest.id.clone(),
        operation: "chat".to_string(),
    };

    let req = ai_lib_rust::protocol::UnifiedRequest {
        operation: "chat".to_string(),
        model: "deepseek-chat".to_string(),
        messages: vec![ai_lib_rust::Message::user("hello")],
        stream: false,
        ..Default::default()
    };

    // Wrap the call with hooks.
    let _resp = pipeline
        .execute(&ctx, &req, || async {
            client.call_model(req.clone()).await
        })
        .await?;

    Ok(())
}

#[cfg(not(feature = "interceptors"))]
fn main() {
    eprintln!("Enable feature: --features interceptors");
}
