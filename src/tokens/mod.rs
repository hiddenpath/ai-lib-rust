//! Token 计数与成本估算模块：提供多种 Token 统计方法和价格计算功能。
//!
//! # Token Counting and Cost Estimation Module
//!
//! This module provides token counting and cost estimation capabilities,
//! essential for managing API budgets and optimizing request sizes.
//!
//! ## Overview
//!
//! Token counting is critical for:
//! - Staying within model context length limits
//! - Estimating API costs before requests
//! - Optimizing prompt lengths for efficiency
//! - Budget tracking and alerts
//!
//! ## Key Components
//!
//! | Component | Description |
//! |-----------|-------------|
//! | [`TokenCounter`] | Trait for token counting implementations |
//! | [`CharacterEstimator`] | Fast character-based approximation (4 chars ≈ 1 token) |
//! | [`AnthropicEstimator`] | Anthropic-specific token estimation |
//! | [`CachingCounter`] | Wrapper that caches token counts |
//! | [`ModelPricing`] | Pricing information per model |
//! | [`CostEstimate`] | Estimated cost breakdown |
//!
//! ## Example
//!
//! ```rust
//! use ai_lib_rust::tokens::{TokenCounter, CharacterEstimator, ModelPricing, CostEstimate};
//!
//! // Count tokens using character estimation
//! let counter = CharacterEstimator::new();
//! let token_count = counter.count_tokens("Hello, how are you?");
//! println!("Estimated tokens: {}", token_count);
//!
//! // Estimate cost
//! let pricing = ModelPricing {
//!     input_cost_per_1k: 0.01,
//!     output_cost_per_1k: 0.03,
//!     ..Default::default()
//! };
//! let estimate = CostEstimate::calculate(token_count, 100, &pricing);
//! println!("Estimated cost: ${:.4}", estimate.total_cost);
//! ```
//!
//! ## Token Estimation Accuracy
//!
//! | Method | Accuracy | Speed | Use Case |
//! |--------|----------|-------|----------|
//! | Character-based | ~85% | Fast | Quick estimates, previews |
//! | Model-specific | ~95%+ | Medium | Accurate budgeting |
//! | Cached | Varies | Fastest | Repeated content |

mod counter;
mod pricing;

pub use counter::{
    get_token_counter, AnthropicEstimator, CachingCounter, CharacterEstimator, TokenCounter,
};
pub use pricing::{CostEstimate, ModelPricing};
