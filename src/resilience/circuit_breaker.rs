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

