//! Request batching module.

mod collector;
mod executor;

pub use collector::{BatchCollector, BatchConfig, BatchItem, BatchAddResult};
pub use executor::{BatchExecutor, BatchExecutorConfig, BatchResult, BatchError, BatchStrategy};
