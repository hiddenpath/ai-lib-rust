//! Multi-provider selection example using routing_mvp + multiple AiClient.
//!
//! Demonstrates how to combine CustomModelManager with multiple providers
//! for model selection and fallback.
//!
//! Run: `cargo run --example multi_provider --features routing_mvp`

#[cfg(feature = "routing_mvp")]
use ai_lib_rust::{
    AiClient, CustomModelManager, Message, ModelCapabilities, ModelInfo, ModelSelectionStrategy,
    PerformanceMetrics, PricingInfo, QualityTier, SpeedTier,
};
#[cfg(feature = "routing_mvp")]
use std::time::Duration;

#[cfg(feature = "routing_mvp")]
#[tokio::main]
async fn main() -> ai_lib_rust::Result<()> {
    println!("🚀 ai-lib-rust multi_provider example");

    // 1. Single client, multi-model via ChatRequestBuilder::model()
    let client = AiClient::new("openai/gpt-4o").await?;

    // 2. Use routing_mvp for selection (pure heuristics)
    let mut mgr = CustomModelManager::new("openai")
        .with_strategy(ModelSelectionStrategy::CostBased);
    mgr.add_model(ModelInfo {
        name: "gpt-4o-mini".to_string(),
        display_name: "GPT-4o Mini".to_string(),
        description: "Fast and cheap".to_string(),
        capabilities: ModelCapabilities::new().with_chat().with_context_window(128000),
        pricing: PricingInfo::new(0.15, 0.60),
        performance: PerformanceMetrics::new()
            .with_speed(SpeedTier::Fast)
            .with_quality(QualityTier::Good)
            .with_avg_response_time(Duration::from_millis(500)),
        metadata: Default::default(),
    });
    mgr.add_model(ModelInfo {
        name: "gpt-4o".to_string(),
        display_name: "GPT-4o".to_string(),
        description: "Higher quality".to_string(),
        capabilities: ModelCapabilities::new().with_chat().with_context_window(128000),
        pricing: PricingInfo::new(2.50, 10.00),
        performance: PerformanceMetrics::new()
            .with_speed(SpeedTier::Balanced)
            .with_quality(QualityTier::Excellent)
            .with_avg_response_time(Duration::from_millis(800)),
        metadata: Default::default(),
    });

    let selected = mgr.select_model().expect("at least one model");

    // 3. Override model per request (single client, multi-model)
    let resp = client
        .chat()
        .messages(vec![Message::user("Say hello in one word")])
        .model(selected.name.as_str())
        .execute()
        .await?;

    println!("Response: {}", resp.content);
    println!("Metrics: {:?}", client.metrics());

    Ok(())
}

#[cfg(not(feature = "routing_mvp"))]
fn main() {
    eprintln!("Enable feature: cargo run --example multi_provider --features routing_mvp");
}
