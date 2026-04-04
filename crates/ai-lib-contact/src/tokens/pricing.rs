//! Model pricing and cost estimation.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub model: String,
    pub input_cost_per_1k: f64,
    pub output_cost_per_1k: f64,
    pub currency: String,
}

impl ModelPricing {
    pub fn new(model: &str, input: f64, output: f64) -> Self {
        Self {
            model: model.into(),
            input_cost_per_1k: input,
            output_cost_per_1k: output,
            currency: "USD".into(),
        }
    }
    pub fn calculate_cost(&self, input_tokens: u32, output_tokens: u32) -> CostEstimate {
        let ic = (input_tokens as f64 / 1000.0) * self.input_cost_per_1k;
        let oc = (output_tokens as f64 / 1000.0) * self.output_cost_per_1k;
        CostEstimate {
            model: self.model.clone(),
            input_tokens,
            output_tokens,
            input_cost: ic,
            output_cost: oc,
            total_cost: ic + oc,
            currency: self.currency.clone(),
        }
    }
    pub fn gpt_4o() -> Self {
        Self::new("gpt-4o", 0.005, 0.015)
    }
    pub fn gpt_4o_mini() -> Self {
        Self::new("gpt-4o-mini", 0.00015, 0.0006)
    }
    pub fn claude_35_sonnet() -> Self {
        Self::new("claude-3-5-sonnet", 0.003, 0.015)
    }
    pub fn claude_3_haiku() -> Self {
        Self::new("claude-3-haiku", 0.00025, 0.00125)
    }
    pub fn for_model(model: &str) -> Option<Self> {
        let m = model.to_lowercase();
        if m.contains("gpt-4o-mini") {
            Some(Self::gpt_4o_mini())
        } else if m.contains("gpt-4o") {
            Some(Self::gpt_4o())
        } else if m.contains("claude-3-5-sonnet") {
            Some(Self::claude_35_sonnet())
        } else if m.contains("claude-3-haiku") {
            Some(Self::claude_3_haiku())
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub model: String,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub currency: String,
}

impl CostEstimate {
    pub fn format(&self) -> String {
        format!("{} {:.6}", self.currency, self.total_cost)
    }
    pub fn format_detailed(&self) -> String {
        if self.total_cost < 0.01 {
            format!("{:.4}¢", self.total_cost * 100.0)
        } else {
            format!("${:.4}", self.total_cost)
        }
    }
}
