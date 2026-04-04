/// A lightweight, provider-agnostic snapshot of runtime "signals" for orchestration.
///
/// This is intentionally *facts only* (no policy). Applications can build scoring/selection
/// strategies on top of these signals.
#[derive(Debug, Clone, Default)]
pub struct SignalsSnapshot {
    pub inflight: Option<InflightSnapshot>,
}

#[derive(Debug, Clone)]
pub struct InflightSnapshot {
    pub max: usize,
    pub available: usize,
    pub in_use: usize,
}
