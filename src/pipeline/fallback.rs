//! Fallback Operator
//!
//! This operator handles failover to alternative models/providers.

use crate::pipeline::PipelineError;
use async_trait::async_trait;

pub struct FallbackOperator {
    pub candidates: Vec<String>, // List of model IDs
}

impl FallbackOperator {
    pub fn new(candidates: Vec<String>) -> Self {
        Self { candidates }
    }

    pub fn next(&self, current_failed_model: &str) -> Option<&str> {
        // Find current model index and return next
        let idx = self
            .candidates
            .iter()
            .position(|r| r == current_failed_model);
        match idx {
            Some(i) => {
                if i + 1 < self.candidates.len() {
                    Some(&self.candidates[i + 1])
                } else {
                    None // End of list
                }
            }
            None => self.candidates.first().map(|s| s.as_str()),
        }
    }
}
