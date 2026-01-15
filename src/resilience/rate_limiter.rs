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
}
