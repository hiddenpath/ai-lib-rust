use crate::{Error, Result};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct CircuitBreakerSnapshot {
    pub failure_threshold: u32,
    pub cooldown_ms: u64,
    pub consecutive_failures: u32,
    /// Remaining open time in ms, if currently open.
    pub open_remaining_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub cooldown: Duration,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            cooldown: Duration::from_secs(30),
        }
    }
}

#[derive(Debug)]
struct State {
    consecutive_failures: u32,
    open_until: Option<Instant>,
}

/// Minimal circuit breaker.
///
/// - Disabled unless configured
/// - Counts consecutive failures
/// - Opens for a cooldown duration after threshold
pub struct CircuitBreaker {
    cfg: CircuitBreakerConfig,
    state: std::sync::Mutex<State>,
}

impl CircuitBreaker {
    pub fn new(cfg: CircuitBreakerConfig) -> Self {
        Self {
            cfg,
            state: std::sync::Mutex::new(State {
                consecutive_failures: 0,
                open_until: None,
            }),
        }
    }

    pub fn allow(&self) -> Result<()> {
        let mut st = self.state.lock().map_err(|_| {
            Error::runtime_with_context(
                "CircuitBreaker poisoned",
                crate::ErrorContext::new().with_source("circuit_breaker"),
            )
        })?;
        if let Some(until) = st.open_until {
            if Instant::now() < until {
                return Err(Error::runtime_with_context(
                    "circuit breaker open",
                    crate::ErrorContext::new().with_source("circuit_breaker"),
                ));
            }
            // cooldown expired
            st.open_until = None;
            st.consecutive_failures = 0;
        }
        Ok(())
    }

    pub fn on_success(&self) {
        if let Ok(mut st) = self.state.lock() {
            st.consecutive_failures = 0;
            st.open_until = None;
        }
    }

    pub fn on_failure(&self) {
        if let Ok(mut st) = self.state.lock() {
            st.consecutive_failures = st.consecutive_failures.saturating_add(1);
            if st.consecutive_failures >= self.cfg.failure_threshold {
                st.open_until = Some(Instant::now() + self.cfg.cooldown);
            }
        }
    }

    pub fn snapshot(&self) -> CircuitBreakerSnapshot {
        let now = Instant::now();
        if let Ok(st) = self.state.lock() {
            let open_remaining_ms = st.open_until.and_then(|until| {
                if until > now {
                    Some((until - now).as_millis() as u64)
                } else {
                    None
                }
            });
            CircuitBreakerSnapshot {
                failure_threshold: self.cfg.failure_threshold,
                cooldown_ms: self.cfg.cooldown.as_millis() as u64,
                consecutive_failures: st.consecutive_failures,
                open_remaining_ms,
            }
        } else {
            CircuitBreakerSnapshot {
                failure_threshold: self.cfg.failure_threshold,
                cooldown_ms: self.cfg.cooldown.as_millis() as u64,
                consecutive_failures: 0,
                open_remaining_ms: None,
            }
        }
    }
}

impl CircuitBreakerConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the failure threshold
    pub fn with_failure_threshold(mut self, threshold: u32) -> Self {
        self.failure_threshold = threshold;
        self
    }

    /// Set the cooldown duration
    pub fn with_cooldown(mut self, cooldown: Duration) -> Self {
        self.cooldown = cooldown;
        self
    }

    /// Alias for with_cooldown for API consistency
    pub fn with_reset_timeout(self, timeout: Duration) -> Self {
        self.with_cooldown(timeout)
    }
}

impl CircuitBreaker {
    /// Check if a request is allowed (alias for allow)
    pub fn allow_request(&self) -> bool {
        self.allow().is_ok()
    }

    /// Record a success (alias for on_success)
    pub fn record_success(&self) {
        self.on_success();
    }

    /// Record a failure (alias for on_failure)
    pub fn record_failure(&self) {
        self.on_failure();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn test_circuit_breaker_config_defaults() {
        let config = CircuitBreakerConfig::default();
        assert_eq!(config.failure_threshold, 5);
        assert_eq!(config.cooldown, Duration::from_secs(30));
    }

    #[test]
    fn test_circuit_breaker_config_builder() {
        let config = CircuitBreakerConfig::new()
            .with_failure_threshold(3)
            .with_cooldown(Duration::from_secs(10));
        assert_eq!(config.failure_threshold, 3);
        assert_eq!(config.cooldown, Duration::from_secs(10));
    }

    #[test]
    fn test_circuit_breaker_initial_state() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert!(cb.allow().is_ok());
        assert!(cb.allow_request());
        
        let snapshot = cb.snapshot();
        assert_eq!(snapshot.consecutive_failures, 0);
        assert!(snapshot.open_remaining_ms.is_none());
    }

    #[test]
    fn test_circuit_breaker_success_resets_failures() {
        let config = CircuitBreakerConfig::new().with_failure_threshold(5);
        let cb = CircuitBreaker::new(config);
        
        // Record some failures
        cb.on_failure();
        cb.on_failure();
        assert_eq!(cb.snapshot().consecutive_failures, 2);
        
        // Success resets counter
        cb.on_success();
        assert_eq!(cb.snapshot().consecutive_failures, 0);
    }

    #[test]
    fn test_circuit_breaker_opens_at_threshold() {
        let config = CircuitBreakerConfig::new()
            .with_failure_threshold(3)
            .with_cooldown(Duration::from_millis(100));
        let cb = CircuitBreaker::new(config);
        
        // Below threshold - still closed
        cb.on_failure();
        cb.on_failure();
        assert!(cb.allow().is_ok());
        
        // At threshold - opens
        cb.on_failure();
        assert!(cb.allow().is_err());
        assert!(cb.snapshot().open_remaining_ms.is_some());
    }

    #[test]
    fn test_circuit_breaker_closes_after_cooldown() {
        let config = CircuitBreakerConfig::new()
            .with_failure_threshold(2)
            .with_cooldown(Duration::from_millis(50));
        let cb = CircuitBreaker::new(config);
        
        // Open the circuit
        cb.on_failure();
        cb.on_failure();
        assert!(cb.allow().is_err());
        
        // Wait for cooldown
        thread::sleep(Duration::from_millis(60));
        
        // Should be closed again
        assert!(cb.allow().is_ok());
        assert_eq!(cb.snapshot().consecutive_failures, 0);
    }

    #[test]
    fn test_circuit_breaker_snapshot() {
        let config = CircuitBreakerConfig::new()
            .with_failure_threshold(5)
            .with_cooldown(Duration::from_secs(30));
        let cb = CircuitBreaker::new(config);
        
        cb.on_failure();
        cb.on_failure();
        
        let snapshot = cb.snapshot();
        assert_eq!(snapshot.failure_threshold, 5);
        assert_eq!(snapshot.cooldown_ms, 30_000);
        assert_eq!(snapshot.consecutive_failures, 2);
        assert!(snapshot.open_remaining_ms.is_none());
    }

    #[test]
    fn test_circuit_breaker_thread_safe() {
        use std::sync::Arc;
        
        let config = CircuitBreakerConfig::new().with_failure_threshold(100);
        let cb = Arc::new(CircuitBreaker::new(config));
        
        let mut handles = vec![];
        for _ in 0..10 {
            let cb_clone = Arc::clone(&cb);
            handles.push(thread::spawn(move || {
                for _ in 0..5 {
                    cb_clone.on_failure();
                }
            }));
        }
        
        for h in handles {
            h.join().unwrap();
        }
        
        assert_eq!(cb.snapshot().consecutive_failures, 50);
    }

    #[test]
    fn test_circuit_breaker_saturating_failures() {
        let config = CircuitBreakerConfig::new().with_failure_threshold(u32::MAX);
        let cb = CircuitBreaker::new(config);
        
        // Record many failures - should not overflow
        for _ in 0..1000 {
            cb.on_failure();
        }
        
        assert_eq!(cb.snapshot().consecutive_failures, 1000);
    }
}
