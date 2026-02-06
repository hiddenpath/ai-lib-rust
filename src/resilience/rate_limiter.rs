use crate::Result;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct RateLimiterSnapshot {
    pub rps: f64,
    pub burst: f64,
    pub tokens: f64,
    /// Estimated wait time until a token is available (ms), if currently empty.
    pub estimated_wait_ms: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Tokens per second.
    pub rps: f64,
    /// Maximum burst size (tokens).
    pub burst: f64,
}

impl RateLimiterConfig {
    pub fn from_rps(rps: f64) -> Option<Self> {
        if !rps.is_finite() || rps < 0.0 {
            return None;
        }
        Some(Self {
            rps,
            burst: rps.max(1.0), // default burst: 1 second worth, at least 1
        })
    }
}

#[derive(Debug)]
struct State {
    tokens: f64,
    last: Instant,
    /// Absolute time when the budget is expected to reset (if blocked by provider)
    blocked_until: Option<Instant>,
    /// Last reported remaining budget from provider
    remaining: Option<u64>,
}

/// Minimal token-bucket rate limiter (opt-in).
///
/// - Default disabled unless configured
/// - Best-effort fairness for async tasks
pub struct RateLimiter {
    cfg: RateLimiterConfig,
    state: Mutex<State>,
}

impl RateLimiter {
    pub fn new(cfg: RateLimiterConfig) -> Self {
        let burst = cfg.burst;
        let state = Mutex::new(State {
            tokens: burst,
            last: Instant::now(),
            blocked_until: None,
            remaining: None,
        });
        Self { cfg, state }
    }

    fn refill_locked(cfg: &RateLimiterConfig, st: &mut State) {
        let now = Instant::now();
        let elapsed = now.duration_since(st.last).as_secs_f64();
        if elapsed > 0.0 {
            st.tokens = (st.tokens + elapsed * cfg.rps).min(cfg.burst);
            st.last = now;
        }
    }

    /// Acquire one token (may sleep).
    pub async fn acquire(&self) -> Result<()> {
        let cfg = &self.cfg;

        loop {
            let wait_duration = {
                let mut st = self.state.lock().await;
                let now = Instant::now();

                // 1. Check if we are explicitly blocked by an external signal
                if let Some(until) = st.blocked_until {
                    if until > now {
                        // Remain in loop and wait
                        until.duration_since(now)
                    } else {
                        st.blocked_until = None;
                        Duration::from_millis(0)
                    }
                } else {
                    if cfg.rps <= 0.0 {
                        return Ok(());
                    }

                    Self::refill_locked(cfg, &mut st);

                    // 2. If we have local tokens and aren't hearing "remaining: 0" from provider, go.
                    if st.tokens >= 1.0 && st.remaining.unwrap_or(1) > 0 {
                        st.tokens -= 1.0;
                        if let Some(rem) = st.remaining.as_mut() {
                            *rem = rem.saturating_sub(1);
                        }
                        return Ok(());
                    }

                    // 3. Compute wait time until next token or reset
                    let missing = 1.0 - st.tokens;
                    Duration::from_secs_f64(missing / cfg.rps)
                }
            };

            if wait_duration.as_millis() > 0 {
                tokio::time::sleep(wait_duration).await;
            }
        }
    }

    /// Update rate limiter state based on external signals (e.g. HTTP headers)
    pub async fn update_budget(
        &self,
        remaining: Option<u64>,
        reset_after: Option<std::time::Duration>,
    ) {
        let mut st = self.state.lock().await;
        if let Some(rem) = remaining {
            st.remaining = Some(rem);
            if rem == 0 {
                // If 0 remaining, we must wait until reset or a default backoff
                let after = reset_after.unwrap_or(std::time::Duration::from_secs(1));
                st.blocked_until = Some(Instant::now() + after);
            } else {
                st.blocked_until = None;
            }
        }
    }

    pub async fn snapshot(&self) -> RateLimiterSnapshot {
        let cfg = &self.cfg;
        let mut st = self.state.lock().await;
        let now = Instant::now();

        // 1. Check external block first
        let mut wait_ms = None;
        if let Some(until) = st.blocked_until {
            if until > now {
                wait_ms = Some(until.duration_since(now).as_millis() as u64);
            }
        }

        // 2. Then check local token bucket if no external block or if longer
        if cfg.rps > 0.0 {
            Self::refill_locked(cfg, &mut st);
            if st.tokens < 1.0 {
                let missing = 1.0 - st.tokens;
                let local_wait_ms = (missing / cfg.rps * 1000.0) as u64;
                wait_ms = Some(wait_ms.unwrap_or(0).max(local_wait_ms));
            }
        }

        RateLimiterSnapshot {
            rps: cfg.rps,
            burst: cfg.burst,
            tokens: st.tokens,
            estimated_wait_ms: wait_ms,
        }
    }

    /// Try to acquire a token without waiting, returns true if successful
    pub async fn try_acquire(&self) -> bool {
        let cfg = &self.cfg;
        if cfg.rps <= 0.0 {
            return true;
        }

        let mut st = self.state.lock().await;
        Self::refill_locked(cfg, &mut st);

        if st.tokens >= 1.0 && st.remaining.unwrap_or(1) > 0 {
            st.tokens -= 1.0;
            if let Some(rem) = st.remaining.as_mut() {
                *rem = rem.saturating_sub(1);
            }
            true
        } else {
            false
        }
    }
}

impl RateLimiterConfig {
    /// Create a new config with default values
    pub fn new() -> Self {
        Self { rps: 10.0, burst: 10.0 }
    }

    /// Set the maximum tokens (burst size)
    pub fn with_max_tokens(mut self, tokens: u32) -> Self {
        self.burst = tokens as f64;
        self
    }

    /// Set the refill rate (tokens per second)
    pub fn with_refill_rate(mut self, rate: f64) -> Self {
        self.rps = rate;
        self
    }
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_config_from_rps() {
        let config = RateLimiterConfig::from_rps(10.0).unwrap();
        assert_eq!(config.rps, 10.0);
        assert_eq!(config.burst, 10.0);
    }

    #[test]
    fn test_rate_limiter_config_from_rps_low() {
        let config = RateLimiterConfig::from_rps(0.5).unwrap();
        assert_eq!(config.rps, 0.5);
        // burst should be at least 1.0
        assert_eq!(config.burst, 1.0);
    }

    #[test]
    fn test_rate_limiter_config_from_rps_invalid() {
        assert!(RateLimiterConfig::from_rps(-1.0).is_none());
        assert!(RateLimiterConfig::from_rps(f64::NAN).is_none());
        assert!(RateLimiterConfig::from_rps(f64::INFINITY).is_none());
    }

    #[test]
    fn test_rate_limiter_config_builder() {
        let config = RateLimiterConfig::new()
            .with_max_tokens(100)
            .with_refill_rate(50.0);
        assert_eq!(config.burst, 100.0);
        assert_eq!(config.rps, 50.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_initial_burst() {
        let config = RateLimiterConfig::from_rps(10.0).unwrap();
        let limiter = RateLimiter::new(config);
        
        // Should have burst tokens available
        let snapshot = limiter.snapshot().await;
        assert_eq!(snapshot.burst, 10.0);
        assert!(snapshot.tokens >= 9.0); // Allow for small timing variations
    }

    #[tokio::test]
    async fn test_rate_limiter_acquire() {
        let config = RateLimiterConfig::from_rps(100.0).unwrap(); // High rate for fast test
        let limiter = RateLimiter::new(config);
        
        // Should succeed for burst
        for _ in 0..10 {
            assert!(limiter.acquire().await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_try_acquire() {
        let config = RateLimiterConfig::new()
            .with_max_tokens(3)
            .with_refill_rate(1.0);
        let limiter = RateLimiter::new(config);
        
        // Should succeed for burst
        assert!(limiter.try_acquire().await);
        assert!(limiter.try_acquire().await);
        assert!(limiter.try_acquire().await);
        
        // Fourth should fail (no tokens left)
        assert!(!limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_zero_rps() {
        let config = RateLimiterConfig::from_rps(0.0).unwrap();
        let limiter = RateLimiter::new(config);
        
        // Zero RPS means unlimited
        assert!(limiter.acquire().await.is_ok());
    }

    #[tokio::test]
    async fn test_rate_limiter_update_budget() {
        let config = RateLimiterConfig::from_rps(10.0).unwrap();
        let limiter = RateLimiter::new(config);
        
        // Set remaining to 0 with reset
        limiter.update_budget(Some(0), Some(Duration::from_millis(50))).await;
        
        let snapshot = limiter.snapshot().await;
        assert!(snapshot.estimated_wait_ms.is_some());
        
        // Wait for reset
        tokio::time::sleep(Duration::from_millis(60)).await;
        
        // Update with tokens available
        limiter.update_budget(Some(10), None).await;
        
        // Should be unblocked
        assert!(limiter.try_acquire().await);
    }

    #[tokio::test]
    async fn test_rate_limiter_snapshot() {
        let config = RateLimiterConfig::new()
            .with_max_tokens(10)
            .with_refill_rate(5.0);
        let limiter = RateLimiter::new(config);
        
        let snapshot = limiter.snapshot().await;
        assert_eq!(snapshot.rps, 5.0);
        assert_eq!(snapshot.burst, 10.0);
        assert!(snapshot.tokens > 0.0);
    }

    #[tokio::test]
    async fn test_rate_limiter_refill() {
        let config = RateLimiterConfig::new()
            .with_max_tokens(5)
            .with_refill_rate(100.0); // 100 tokens/sec = 1 token/10ms
        let limiter = RateLimiter::new(config);
        
        // Consume all tokens
        for _ in 0..5 {
            assert!(limiter.try_acquire().await);
        }
        
        // Should be empty
        assert!(!limiter.try_acquire().await);
        
        // Wait for refill (10ms = 1 token at 100 rps)
        tokio::time::sleep(Duration::from_millis(20)).await;
        
        // Should have at least 1 token now
        assert!(limiter.try_acquire().await);
    }
}
