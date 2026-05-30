//! Deterministic context budget assembly (ALR-P2-001). No network, no LLM summarization.

mod assembler;
mod budget;
mod error;
mod token_estimate;

pub use assembler::{AssembleOptions, AssembleReport, MessageAssembler};
pub use budget::{ContextBudget, ModelCapacity};
pub use error::AssembleError;
pub use token_estimate::{estimate_message_tokens, estimate_tokens, CHARS_PER_TOKEN};
