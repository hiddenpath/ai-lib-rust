//! 弹性模式模块：提供熔断器和限流器等可靠性保障机制。
//!
//! # Resilience Primitives Module
//!
//! This module provides opt-in resilience patterns for building robust AI applications
//! that gracefully handle failures and protect against overload.
//!
//! ## Overview
//!
//! Resilience patterns are essential for production AI systems to:
//! - Prevent cascade failures when downstream services are unavailable
//! - Protect against API rate limit violations
//! - Provide graceful degradation under high load
//! - Enable fast failure detection and recovery
//!
//! ## Key Components
//!
//! | Component | Description |
//! |-----------|-------------|
//! | [`circuit_breaker`] | Circuit breaker pattern for failure isolation |
//! | [`rate_limiter`] | Token bucket rate limiter for throughput control |
//!
//! ## Circuit Breaker
//!
//! The circuit breaker prevents repeated calls to a failing service:
//! - **Closed**: Normal operation, requests pass through
//! - **Open**: Failures exceeded threshold, requests fail fast
//! - **Half-Open**: Testing if service has recovered
//!
//! ```rust
//! use ai_lib_rust::resilience::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig};
//! use std::time::Duration;
//!
//! let config = CircuitBreakerConfig::new()
//!     .with_failure_threshold(5)
//!     .with_reset_timeout(Duration::from_secs(30));
//! let breaker = CircuitBreaker::new(config);
//!
//! // Check if request is allowed
//! if breaker.allow_request() {
//!     // Make API call...
//!     breaker.record_success();
//! }
//! ```
//!
//! ## Rate Limiter
//!
//! The rate limiter controls request throughput using the token bucket algorithm:
//!
//! ```rust
//! use ai_lib_rust::resilience::rate_limiter::{RateLimiter, RateLimiterConfig};
//!
//! let config = RateLimiterConfig::new()
//!     .with_max_tokens(100)
//!     .with_refill_rate(10.0); // 10 tokens per second
//! let limiter = RateLimiter::new(config);
//!
//! // Try to acquire a permit
//! if limiter.try_acquire(1) {
//!     // Proceed with request...
//! }
//! ```

pub mod circuit_breaker;
pub mod rate_limiter;
