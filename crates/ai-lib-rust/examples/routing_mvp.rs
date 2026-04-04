//! Model management / routing MVP example.
//!
//! Run:
//! - `cargo run --example routing_mvp --features routing_mvp`

#[cfg(feature = "routing_mvp")]
use ai_lib_rust::{
    CustomModelManager, ModelCapabilities, ModelInfo, ModelSelectionStrategy, PerformanceMetrics,
    PricingInfo, QualityTier, SpeedTier,
};

#[cfg(feature = "routing_mvp")]
use std::time::Duration;

#[cfg(feature = "routing_mvp")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 ai-lib-rust routing_mvp example");

    // Pure logic: select a provider-native model name based on heuristics.
    let mut mgr =
        CustomModelManager::new("groq").with_strategy(ModelSelectionStrategy::PerformanceBased);

    mgr.add_model(ModelInfo {
        name: "llama-3.3-70b-versatile".to_string(),
        display_name: "Llama 3.3 70B".to_string(),
        description: "High performance general-purpose chat model".to_string(),
        capabilities: ModelCapabilities::new()
            .with_chat()
            .with_context_window(131072),
        pricing: PricingInfo::new(0.59, 0.79),
        performance: PerformanceMetrics::new()
            .with_speed(SpeedTier::Balanced)
            .with_quality(QualityTier::Excellent)
            .with_avg_response_time(Duration::from_millis(900)),
        metadata: Default::default(),
    });

    mgr.add_model(ModelInfo {
        name: "llama-3.1-8b-instant".to_string(),
        display_name: "Llama 3.1 8B".to_string(),
        description: "Fast and cheap model for simple tasks".to_string(),
        capabilities: ModelCapabilities::new()
            .with_chat()
            .with_context_window(8192),
        pricing: PricingInfo::new(0.05, 0.08),
        performance: PerformanceMetrics::new()
            .with_speed(SpeedTier::Fast)
            .with_quality(QualityTier::Good)
            .with_avg_response_time(Duration::from_millis(300)),
        metadata: Default::default(),
    });

    let selected = mgr.select_model().expect("at least one model");
    println!(
        "Selected model: {} ({})",
        selected.display_name, selected.name
    );

    // Runtime style: once you have a model choice, you build a client with `provider/model`.
    let model_id = format!("{}/{}", mgr.provider, selected.name);
    println!("Runtime model_id: {}", model_id);

    Ok(())
}

#[cfg(not(feature = "routing_mvp"))]
fn main() {
    eprintln!("Enable feature: --features routing_mvp");
}
