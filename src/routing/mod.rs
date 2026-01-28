//! Optional model management helpers (routing MVP).
//!
//! This module is intentionally **pure logic**: it doesn't perform network calls and does not
//! depend on any provider SDK. It can be used by applications to select a `model_id`
//! (e.g. `"groq/llama-3.3-70b-versatile"`) before building an `AiClient`.
//!
//! Design note (runtime-first):
//! - In runtime style, providers/models are configured via AI-Protocol manifests.
//! - These helpers focus on selection and bookkeeping only.

use crate::{Error, ErrorContext, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;

/// Model information structure for custom model management.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model name/identifier (usually provider-native model name, e.g. "gpt-4o").
    pub name: String,
    /// Display name for user interface.
    pub display_name: String,
    /// Model description.
    pub description: String,
    /// Model capabilities.
    pub capabilities: ModelCapabilities,
    /// Pricing information.
    pub pricing: PricingInfo,
    /// Performance metrics.
    pub performance: PerformanceMetrics,
    /// Provider-specific metadata (free-form).
    pub metadata: HashMap<String, String>,
}

/// Model capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelCapabilities {
    pub chat: bool,
    pub code_generation: bool,
    pub multimodal: bool,
    pub function_calling: bool,
    pub tool_use: bool,
    pub multilingual: bool,
    pub context_window: Option<u32>,
}

impl ModelCapabilities {
    pub fn new() -> Self {
        Self {
            chat: true,
            code_generation: false,
            multimodal: false,
            function_calling: false,
            tool_use: false,
            multilingual: false,
            context_window: None,
        }
    }

    pub fn with_chat(mut self) -> Self {
        self.chat = true;
        self
    }

    pub fn with_code_generation(mut self) -> Self {
        self.code_generation = true;
        self
    }

    pub fn with_multimodal(mut self) -> Self {
        self.multimodal = true;
        self
    }

    pub fn with_function_calling(mut self) -> Self {
        self.function_calling = true;
        self
    }

    pub fn with_tool_use(mut self) -> Self {
        self.tool_use = true;
        self
    }

    pub fn with_multilingual(mut self) -> Self {
        self.multilingual = true;
        self
    }

    pub fn with_context_window(mut self, size: u32) -> Self {
        self.context_window = Some(size);
        self
    }

    pub fn supports(&self, capability: &str) -> bool {
        match capability {
            "chat" => self.chat,
            "code_generation" => self.code_generation,
            "multimodal" => self.multimodal,
            "function_calling" => self.function_calling,
            "tool_use" => self.tool_use,
            "multilingual" => self.multilingual,
            _ => false,
        }
    }
}

impl Default for ModelCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

/// Pricing information for models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PricingInfo {
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
    pub currency: String,
}

impl PricingInfo {
    pub fn new(input_cost_per_1k: f64, output_cost_per_1k: f64) -> Self {
        Self {
            input_cost_per_1k,
            output_cost_per_1k,
            currency: "USD".to_string(),
        }
    }

    pub fn with_currency(mut self, currency: &str) -> Self {
        self.currency = currency.to_string();
        self
    }

    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> f64 {
        let input_cost = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let output_cost = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        input_cost + output_cost
    }
}

/// Performance metrics for models.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub speed: SpeedTier,
    pub quality: QualityTier,
    pub avg_response_time: Option<Duration>,
    pub throughput: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpeedTier {
    Fast,
    Balanced,
    Slow,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum QualityTier {
    Basic,
    Good,
    Excellent,
}

impl PerformanceMetrics {
    pub fn new() -> Self {
        Self {
            speed: SpeedTier::Balanced,
            quality: QualityTier::Good,
            avg_response_time: None,
            throughput: None,
        }
    }

    pub fn with_speed(mut self, speed: SpeedTier) -> Self {
        self.speed = speed;
        self
    }

    pub fn with_quality(mut self, quality: QualityTier) -> Self {
        self.quality = quality;
        self
    }

    pub fn with_avg_response_time(mut self, time: Duration) -> Self {
        self.avg_response_time = Some(time);
        self
    }

    pub fn with_throughput(mut self, tps: f64) -> Self {
        self.throughput = Some(tps);
        self
    }
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Model selection strategies.
#[derive(Debug, Clone)]
pub enum ModelSelectionStrategy {
    RoundRobin,
    Weighted,
    LeastConnections,
    PerformanceBased,
    CostBased,
}

/// Custom model manager for applications.
#[derive(Clone)]
pub struct CustomModelManager {
    pub provider: String,
    pub models: HashMap<String, ModelInfo>,
    pub selection_strategy: ModelSelectionStrategy,
}

impl CustomModelManager {
    pub fn new(provider: &str) -> Self {
        Self {
            provider: provider.to_string(),
            models: HashMap::new(),
            selection_strategy: ModelSelectionStrategy::RoundRobin,
        }
    }

    pub fn add_model(&mut self, model: ModelInfo) {
        self.models.insert(model.name.clone(), model);
    }

    pub fn remove_model(&mut self, model_name: &str) -> Option<ModelInfo> {
        self.models.remove(model_name)
    }

    pub fn get_model(&self, model_name: &str) -> Option<&ModelInfo> {
        self.models.get(model_name)
    }

    pub fn list_models(&self) -> Vec<&ModelInfo> {
        self.models.values().collect()
    }

    pub fn with_strategy(mut self, strategy: ModelSelectionStrategy) -> Self {
        self.selection_strategy = strategy;
        self
    }

    /// Select a model by strategy (stateless heuristic).
    ///
    /// Note: for a production-grade, deterministic round-robin across threads,
    /// prefer maintaining an atomic counter in the application layer. This MVP is
    /// intentionally lightweight.
    pub fn select_model(&self) -> Option<&ModelInfo> {
        if self.models.is_empty() {
            return None;
        }

        match self.selection_strategy {
            ModelSelectionStrategy::RoundRobin => {
                let models: Vec<&ModelInfo> = self.models.values().collect();
                let index = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as usize)
                    % models.len();
                Some(models[index])
            }
            ModelSelectionStrategy::Weighted => self.models.values().max_by_key(|model| {
                let speed_score = match model.performance.speed {
                    SpeedTier::Fast => 3,
                    SpeedTier::Balanced => 2,
                    SpeedTier::Slow => 1,
                };
                let quality_score = match model.performance.quality {
                    QualityTier::Excellent => 3,
                    QualityTier::Good => 2,
                    QualityTier::Basic => 1,
                };
                speed_score + quality_score
            }),
            ModelSelectionStrategy::LeastConnections => self.models.values().next(),
            ModelSelectionStrategy::PerformanceBased => {
                self.models
                    .values()
                    .max_by_key(|model| match model.performance.speed {
                        SpeedTier::Fast => 3,
                        SpeedTier::Balanced => 2,
                        SpeedTier::Slow => 1,
                    })
            }
            ModelSelectionStrategy::CostBased => self.models.values().min_by(|a, b| {
                let a_cost = a.pricing.input_cost_per_1k + a.pricing.output_cost_per_1k;
                let b_cost = b.pricing.input_cost_per_1k + b.pricing.output_cost_per_1k;
                a_cost
                    .partial_cmp(&b_cost)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
    }

    pub fn recommend_for(&self, use_case: &str) -> Option<&ModelInfo> {
        let supported_models: Vec<&ModelInfo> = self
            .models
            .values()
            .filter(|model| model.capabilities.supports(use_case))
            .collect();

        supported_models.first().copied()
    }

    pub fn load_from_config(&mut self, config_path: &str) -> Result<()> {
        let content = std::fs::read_to_string(config_path).map_err(|e| {
            Error::configuration_with_context(
                format!("Failed to read config: {}", e),
                ErrorContext::new().with_source("routing_mvp"),
            )
        })?;
        let models: Vec<ModelInfo> = serde_json::from_str(&content)?;
        for model in models {
            self.add_model(model);
        }
        Ok(())
    }

    pub fn save_to_config(&self, config_path: &str) -> Result<()> {
        let models: Vec<&ModelInfo> = self.models.values().collect();
        let content = serde_json::to_string_pretty(&models)?;
        std::fs::write(config_path, content).map_err(|e| {
            Error::configuration_with_context(
                format!("Failed to write config: {}", e),
                ErrorContext::new().with_source("routing_mvp"),
            )
        })?;
        Ok(())
    }
}

/// Load balancing strategies.
#[derive(Debug, Clone)]
pub enum LoadBalancingStrategy {
    RoundRobin,
    Weighted,
    LeastConnections,
    HealthBased,
}

/// Health check configuration for endpoints.
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    pub endpoint: String,
    pub interval: Duration,
    pub timeout: Duration,
    pub max_failures: u32,
}

/// Model endpoint in a model array.
#[derive(Debug, Clone)]
pub struct ModelEndpoint {
    pub name: String,
    /// Provider-native model name.
    pub model_name: String,
    /// Endpoint URL (base URL).
    pub url: String,
    pub weight: f32,
    pub healthy: bool,
    pub connection_count: u32,
}

/// Model array for load balancing / A-B experiments.
#[derive(Clone)]
pub struct ModelArray {
    pub name: String,
    pub endpoints: Vec<ModelEndpoint>,
    pub strategy: LoadBalancingStrategy,
    pub health_check: HealthCheckConfig,
}

impl ModelArray {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            endpoints: Vec::new(),
            strategy: LoadBalancingStrategy::RoundRobin,
            health_check: HealthCheckConfig {
                endpoint: "/health".to_string(),
                interval: Duration::from_secs(30),
                timeout: Duration::from_secs(5),
                max_failures: 3,
            },
        }
    }

    pub fn add_endpoint(&mut self, endpoint: ModelEndpoint) {
        self.endpoints.push(endpoint);
    }

    pub fn with_strategy(mut self, strategy: LoadBalancingStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    pub fn with_health_check(mut self, config: HealthCheckConfig) -> Self {
        self.health_check = config;
        self
    }

    pub fn select_endpoint(&mut self) -> Option<&mut ModelEndpoint> {
        if self.endpoints.is_empty() {
            return None;
        }

        let healthy_indices: Vec<usize> = self
            .endpoints
            .iter()
            .enumerate()
            .filter(|(_, endpoint)| endpoint.healthy)
            .map(|(index, _)| index)
            .collect();

        if healthy_indices.is_empty() {
            return None;
        }

        match self.strategy {
            LoadBalancingStrategy::RoundRobin => {
                let index = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as usize)
                    % healthy_indices.len();
                let endpoint_index = healthy_indices[index];
                Some(&mut self.endpoints[endpoint_index])
            }
            LoadBalancingStrategy::Weighted => {
                let total_weight: f32 = healthy_indices
                    .iter()
                    .map(|&idx| self.endpoints[idx].weight)
                    .sum();
                let mut current_weight = 0.0;

                for &idx in &healthy_indices {
                    current_weight += self.endpoints[idx].weight;
                    if current_weight >= total_weight / 2.0 {
                        return Some(&mut self.endpoints[idx]);
                    }
                }

                Some(&mut self.endpoints[healthy_indices[0]])
            }
            LoadBalancingStrategy::LeastConnections => healthy_indices
                .iter()
                .min_by_key(|&&idx| self.endpoints[idx].connection_count)
                .map(|&idx| &mut self.endpoints[idx]),
            LoadBalancingStrategy::HealthBased => Some(&mut self.endpoints[healthy_indices[0]]),
        }
    }

    pub fn mark_unhealthy(&mut self, endpoint_name: &str) {
        if let Some(endpoint) = self.endpoints.iter_mut().find(|e| e.name == endpoint_name) {
            endpoint.healthy = false;
        }
    }

    pub fn mark_healthy(&mut self, endpoint_name: &str) {
        if let Some(endpoint) = self.endpoints.iter_mut().find(|e| e.name == endpoint_name) {
            endpoint.healthy = true;
        }
    }

    pub fn is_healthy(&self) -> bool {
        self.endpoints.iter().any(|endpoint| endpoint.healthy)
    }
}
