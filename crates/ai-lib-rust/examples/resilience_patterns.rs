//! Resilience Patterns Example
//!
//! This example demonstrates production-ready resilience patterns including:
//! - Circuit breaker for fault isolation
//! - Rate limiting for API protection
//! - Combining multiple resilience strategies
//!
//! These patterns are essential for building robust AI applications that can
//! handle transient failures and protect against cascading failures.
//!
//! Usage:
//!   cargo run --example resilience_patterns

use ai_lib_rust::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
use ai_lib_rust::resilience::rate_limiter::{RateLimiter, RateLimiterConfig};
use std::time::Duration;

#[tokio::main]
async fn main() {
    println!("=== AI-Lib Resilience Patterns Demo ===\n");

    // Example 1: Circuit Breaker
    demo_circuit_breaker();

    // Example 2: Rate Limiter
    demo_rate_limiter().await;

    // Example 3: Combined Patterns
    demo_combined_patterns().await;
}

fn demo_circuit_breaker() {
    println!("--- Example 1: Circuit Breaker ---\n");

    // Configure circuit breaker
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        cooldown: Duration::from_secs(5),
    };

    let circuit_breaker = CircuitBreaker::new(config);

    println!("Circuit Breaker Configuration:");
    println!("  - Failure threshold: 3");
    println!("  - Cooldown: 5 seconds\n");

    // Simulate operations
    println!("Simulating operations...\n");

    // Successful operations
    for i in 1..=2 {
        if circuit_breaker.allow().is_ok() {
            println!("Request {}: Allowed (simulating success)", i);
            circuit_breaker.on_success();
        }
    }

    // Failed operations - will trip the circuit
    for i in 3..=5 {
        if circuit_breaker.allow().is_ok() {
            println!("Request {}: Allowed (simulating failure)", i);
            circuit_breaker.on_failure();
        } else {
            println!("Request {}: BLOCKED by circuit breaker", i);
        }
    }

    // Circuit should be open now
    println!("\nCircuit state after failures:");
    if circuit_breaker.allow().is_err() {
        println!("  Circuit is OPEN - requests are blocked");
    }

    println!("\nNote: Circuit will close after 5 second cooldown\n");
}

async fn demo_rate_limiter() {
    println!("--- Example 2: Rate Limiter ---\n");

    // Create a rate limiter: 5 requests per second
    let config = RateLimiterConfig::from_rps(5.0).expect("Valid RPS");
    let rate_limiter = RateLimiter::new(config);

    println!("Rate Limiter Configuration:");
    println!("  - Rate: 5 requests/second");
    println!("  - Burst capacity: 5\n");

    println!("Attempting rapid requests (acquire will wait if needed)...\n");

    for i in 1..=8 {
        let start = std::time::Instant::now();
        match rate_limiter.acquire().await {
            Ok(_) => {
                let elapsed = start.elapsed();
                if elapsed.as_millis() > 10 {
                    println!("Request {}: Allowed (waited {:?})", i, elapsed);
                } else {
                    println!("Request {}: Allowed immediately", i);
                }
            }
            Err(e) => println!("Request {}: Error: {}", i, e),
        }
    }
    println!();
}

async fn demo_combined_patterns() {
    println!("--- Example 3: Combined Resilience Patterns ---\n");

    // In production, you would combine these patterns
    let cb_config = CircuitBreakerConfig {
        failure_threshold: 5,
        cooldown: Duration::from_secs(30),
    };
    let circuit_breaker = CircuitBreaker::new(cb_config);

    let rl_config = RateLimiterConfig::from_rps(10.0).expect("Valid RPS");
    let rate_limiter = RateLimiter::new(rl_config);

    println!("Production Setup:");
    println!("  - Circuit Breaker: 5 failures -> open, 30s cooldown");
    println!("  - Rate Limiter: 10 RPS\n");

    // Simulate production request flow
    println!("Simulating production request flow...\n");

    for i in 1..=10 {
        // First check rate limiter
        if rate_limiter.acquire().await.is_err() {
            println!("Request {}: REJECTED by rate limiter", i);
            continue;
        }

        // Then check circuit breaker
        if circuit_breaker.allow().is_err() {
            println!("Request {}: REJECTED by circuit breaker (open)", i);
            continue;
        }

        // Simulate request processing
        let success = i % 3 != 0; // Fail every 3rd request

        if success {
            println!("Request {}: SUCCESS", i);
            circuit_breaker.on_success();
        } else {
            println!("Request {}: FAILED (recorded)", i);
            circuit_breaker.on_failure();
        }
    }

    println!("\n=== Best Practices ===\n");
    println!("1. Always use rate limiting for external API calls");
    println!("2. Wrap remote calls with circuit breakers");
    println!("3. Implement exponential backoff for retries");
    println!("4. Monitor circuit breaker state for alerting");
    println!("5. Tune thresholds based on your SLOs");
}
