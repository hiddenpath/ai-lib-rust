//! Batch Processing Example
//!
//! This example demonstrates the batch processing capabilities of ai-lib-rust:
//! - BatchCollector for grouping requests
//! - BatchConfig for configuration
//! - BatchItem for wrapping request data
//! - BatchExecutor for processing batches
//!
//! Batch processing is essential for optimizing throughput when dealing with
//! multiple AI requests that can be processed together.
//!
//! Usage:
//!   cargo run --example batch_processing

use ai_lib_rust::batch::{
    BatchCollector, BatchConfig, BatchItem, BatchExecutor, BatchStrategy,
};
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("=== AI-Lib Batch Processing Demo ===\n");

    // Example 1: Basic Batch Collection
    demo_batch_collector();

    // Example 2: Batch Configuration
    demo_batch_configuration();

    // Example 3: Batch Executor
    demo_batch_executor();

    // Example 4: Batch Strategies
    demo_batch_strategies();
}

fn demo_batch_collector() {
    println!("--- Example 1: Basic Batch Collection ---\n");

    // Create a batch collector with default config
    let config = BatchConfig::new().with_max_batch_size(5);
    let collector: BatchCollector<String> = BatchCollector::new(config);

    println!("BatchCollector created with max size: 5\n");

    // Simulate adding requests
    let requests = vec![
        "Generate summary for doc1",
        "Generate summary for doc2",
        "Generate summary for doc3",
        "Generate summary for doc4",
        "Generate summary for doc5",
    ];

    for (i, request) in requests.iter().enumerate() {
        // Use add_data for simple data addition
        let result = collector.add_data(request.to_string());
        println!("Added request {}: {:?}", i + 1, result);

        // Check if we should flush
        if collector.should_flush() {
            println!("\n  -> Batch is ready to flush!");
            
            // Drain the batch
            let items = collector.drain();
            println!("  -> Drained {} items", items.len());
            
            for item in &items {
                println!("     Processing: {}", item.data);
            }
        }
    }
    println!();
}

fn demo_batch_configuration() {
    println!("--- Example 2: Batch Configuration ---\n");

    // Different configuration options
    let configs = vec![
        ("Small batch, quick flush", BatchConfig::new()
            .with_max_batch_size(3)
            .with_auto_flush(true)),
        ("Large batch, no auto-flush", BatchConfig::new()
            .with_max_batch_size(100)
            .with_auto_flush(false)),
    ];

    for (name, config) in configs {
        println!("Config: {}", name);
        println!("  - Max batch size: {}", config.max_batch_size);
        println!("  - Auto flush: {}", config.auto_flush);
        println!();
    }
}

fn demo_batch_executor() {
    println!("--- Example 3: Batch Executor ---\n");

    // Create executor with default configuration
    let executor = BatchExecutor::new();
    println!("BatchExecutor created\n");

    // Demonstrate batch items with priority
    let items = vec![
        BatchItem::new("High priority request")
            .with_priority(10)
            .with_request_id("req-001"),
        BatchItem::new("Normal priority request")
            .with_priority(5)
            .with_request_id("req-002"),
        BatchItem::new("Low priority request")
            .with_priority(1)
            .with_request_id("req-003"),
    ];

    println!("Created batch items with different priorities:");
    for item in &items {
        println!("  - {:?}: priority={}, request_id={:?}", 
            item.data, item.priority, item.request_id);
    }

    println!("\nNote: In production, executor would process these items");
    println!("      according to the configured strategy.\n");
    
    // Show executor exists
    let _ = executor;
}

fn demo_batch_strategies() {
    println!("--- Example 4: Batch Strategies ---\n");

    // Available batch strategies
    let strategies = vec![
        ("Sequential", BatchStrategy::Sequential),
        ("Parallel", BatchStrategy::Parallel),
        ("Concurrent (5)", BatchStrategy::Concurrent { max_concurrency: 5 }),
    ];

    for (name, strategy) in strategies {
        println!("Strategy: {}", name);
        match strategy {
            BatchStrategy::Sequential => {
                println!("  - Processes items one at a time");
                println!("  - Preserves order, simpler error handling");
                println!("  - Best for: dependent operations, limited concurrency");
            }
            BatchStrategy::Parallel => {
                println!("  - Processes all items concurrently");
                println!("  - Maximum throughput, no concurrency limit");
                println!("  - Best for: independent operations, high throughput needs");
            }
            BatchStrategy::Concurrent { max_concurrency } => {
                println!("  - Processes up to {} items concurrently", max_concurrency);
                println!("  - Controlled parallelism with bounded resources");
                println!("  - Best for: rate-limited APIs, resource-constrained environments");
            }
        }
        println!();
    }

    // Demonstrate time-based batching
    println!("Time-based Batching:\n");
    println!("BatchConfig supports max_wait_time for time-triggered flushes.");
    println!("Default: {:?}\n", Duration::from_secs(5));

    let config = BatchConfig {
        max_batch_size: 10,
        max_wait_time: Duration::from_millis(100),
        auto_flush: true,
    };

    println!("Example config for low-latency batching:");
    println!("  - max_batch_size: {}", config.max_batch_size);
    println!("  - max_wait_time: {:?}", config.max_wait_time);
    println!("  - auto_flush: {}", config.auto_flush);

    println!("\n=== Best Practices ===\n");
    println!("1. Choose batch size based on API limits and memory");
    println!("2. Use time-based flush for consistent latency");
    println!("3. Consider priority for mixed workloads");
    println!("4. Monitor batch fill rates for optimization");
    println!("5. Handle partial batch failures gracefully");
}
