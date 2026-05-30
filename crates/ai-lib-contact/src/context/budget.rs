use serde::{Deserialize, Serialize};

/// Declared model capacity from manifest (`metadata.models` / PT-075). `0` = unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelCapacity {
    pub context_window: u32,
    pub max_output_tokens: u32,
}

impl ModelCapacity {
    pub const UNKNOWN: Self = Self {
        context_window: 0,
        max_output_tokens: 0,
    };

    pub fn new(context_window: u32, max_output_tokens: u32) -> Self {
        Self {
            context_window,
            max_output_tokens,
        }
    }

    pub fn context_window_is_unknown(&self) -> bool {
        self.context_window == 0
    }
}

/// Input-side token budget before calling the provider.
///
/// Token counting priority (see `token_estimate`):
/// 1. Optional caller-supplied `last_usage_prompt_tokens` when assembling the next turn
/// 2. Heuristic char-based estimate (~4 chars / token)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextBudget {
    pub max_input_tokens: u32,
    pub reserve_output_tokens: u32,
    /// Always try to keep at least this many trailing messages when truncating.
    pub min_tail_messages: usize,
}

impl ContextBudget {
    pub const DEFAULT_FALLBACK_INPUT: u32 = 24_000;
    pub const DEFAULT_FALLBACK_OUTPUT_RESERVE: u32 = 4_096;

    pub fn new(max_input_tokens: u32, reserve_output_tokens: u32, min_tail_messages: usize) -> Self {
        Self {
            max_input_tokens,
            reserve_output_tokens,
            min_tail_messages,
        }
    }

    /// Derive input budget from declared capacity minus reserved completion headroom.
    pub fn from_capacity(capacity: ModelCapacity, min_tail_messages: usize) -> Self {
        let reserve = if capacity.max_output_tokens > 0 {
            capacity.max_output_tokens
        } else {
            Self::DEFAULT_FALLBACK_OUTPUT_RESERVE
        };

        let max_input = if capacity.context_window > reserve {
            capacity.context_window - reserve
        } else if capacity.context_window > 0 {
            capacity.context_window
        } else {
            Self::DEFAULT_FALLBACK_INPUT
        };

        Self {
            max_input_tokens: max_input,
            reserve_output_tokens: reserve,
            min_tail_messages,
        }
    }
}
