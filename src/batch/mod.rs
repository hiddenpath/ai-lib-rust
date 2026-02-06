//! 请求批处理模块：提供高效的批量请求收集和执行功能。
//!
//! # Request Batching Module
//!
//! This module provides efficient request batching capabilities for optimizing
//! throughput when dealing with multiple AI requests that can be processed together.
//!
//! ## Overview
//!
//! Batching is essential for:
//! - Reducing API call overhead by grouping multiple requests
//! - Optimizing network utilization with concurrent execution
//! - Managing rate limits through controlled batch sizes
//! - Improving overall throughput for bulk operations
//!
//! ## Key Components
//!
//! | Component | Description |
//! |-----------|-------------|
//! | [`BatchCollector`] | Collects requests until batch criteria are met |
//! | [`BatchConfig`] | Configuration for batch size, timing, and auto-flush |
//! | [`BatchItem`] | Wrapper for individual batch items with metadata |
//! | [`BatchExecutor`] | Executes batches with configurable strategies |
//! | [`BatchStrategy`] | Execution strategy (Sequential, Parallel, Concurrent) |
//!
//! ## Example
//!
//! ```rust
//! use ai_lib_rust::batch::{BatchCollector, BatchConfig, BatchItem};
//!
//! // Create a collector with max batch size of 10
//! let config = BatchConfig::new().with_max_batch_size(10);
//! let collector: BatchCollector<String> = BatchCollector::new(config);
//!
//! // Add items to the batch
//! collector.add_data("request_1".to_string());
//! collector.add_data("request_2".to_string());
//!
//! // Check if batch should be flushed
//! if collector.should_flush() {
//!     let items = collector.drain();
//!     // Process items...
//! }
//! ```
//!
//! ## Strategies
//!
//! - **Sequential**: Process items one at a time, preserving order
//! - **Parallel**: Process all items concurrently with no limit
//! - **Concurrent**: Process up to N items concurrently (recommended for rate-limited APIs)

mod collector;
mod executor;

pub use collector::{BatchAddResult, BatchCollector, BatchConfig, BatchItem};
pub use executor::{BatchError, BatchExecutor, BatchExecutorConfig, BatchResult, BatchStrategy};
