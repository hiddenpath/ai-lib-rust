//! Batch executor.

use super::collector::BatchItem;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BatchResult<T, E> {
    pub successes: Vec<(usize, T)>,
    pub failures: Vec<(usize, E)>,
    pub execution_time: Duration,
    pub total_processed: usize,
}

impl<T, E> BatchResult<T, E> {
    pub fn new() -> Self {
        Self {
            successes: Vec::new(),
            failures: Vec::new(),
            execution_time: Duration::ZERO,
            total_processed: 0,
        }
    }
    pub fn add_success(&mut self, i: usize, r: T) {
        self.successes.push((i, r));
    }
    pub fn add_failure(&mut self, i: usize, e: E) {
        self.failures.push((i, e));
    }
    pub fn all_succeeded(&self) -> bool {
        self.failures.is_empty()
    }
    pub fn success_count(&self) -> usize {
        self.successes.len()
    }
    pub fn failure_count(&self) -> usize {
        self.failures.len()
    }
    pub fn success_rate(&self) -> f64 {
        if self.total_processed == 0 {
            0.0
        } else {
            self.successes.len() as f64 / self.total_processed as f64
        }
    }
}
impl<T, E> Default for BatchResult<T, E> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct BatchError {
    pub message: String,
    pub index: usize,
    pub retryable: bool,
}
impl BatchError {
    pub fn new(msg: impl Into<String>, idx: usize) -> Self {
        Self {
            message: msg.into(),
            index: idx,
            retryable: false,
        }
    }
    pub fn retryable(mut self) -> Self {
        self.retryable = true;
        self
    }
}
impl std::fmt::Display for BatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Batch error at {}: {}", self.index, self.message)
    }
}
impl std::error::Error for BatchError {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatchStrategy {
    Parallel,
    Sequential,
    Concurrent { max_concurrency: usize },
}
impl Default for BatchStrategy {
    fn default() -> Self {
        BatchStrategy::Concurrent { max_concurrency: 5 }
    }
}

#[derive(Debug, Clone)]
pub struct BatchExecutorConfig {
    pub strategy: BatchStrategy,
    pub continue_on_error: bool,
    pub item_timeout: Option<Duration>,
    pub max_retries: u32,
}
impl Default for BatchExecutorConfig {
    fn default() -> Self {
        Self {
            strategy: BatchStrategy::default(),
            continue_on_error: true,
            item_timeout: Some(Duration::from_secs(60)),
            max_retries: 2,
        }
    }
}
impl BatchExecutorConfig {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_strategy(mut self, s: BatchStrategy) -> Self {
        self.strategy = s;
        self
    }
    pub fn with_continue_on_error(mut self, c: bool) -> Self {
        self.continue_on_error = c;
        self
    }
}

pub struct BatchExecutor {
    config: BatchExecutorConfig,
}
impl BatchExecutor {
    pub fn new() -> Self {
        Self {
            config: BatchExecutorConfig::default(),
        }
    }
    pub fn with_config(config: BatchExecutorConfig) -> Self {
        Self { config }
    }
    pub fn config(&self) -> &BatchExecutorConfig {
        &self.config
    }

    pub async fn execute_sequential<T, R, E, F, Fut>(
        &self,
        items: Vec<BatchItem<T>>,
        executor_fn: F,
    ) -> BatchResult<R, BatchError>
    where
        F: Fn(T) -> Fut,
        Fut: std::future::Future<Output = std::result::Result<R, E>>,
        E: std::fmt::Display,
    {
        let start = Instant::now();
        let total = items.len();
        let mut result = BatchResult::new();
        for (i, item) in items.into_iter().enumerate() {
            match executor_fn(item.data).await {
                Ok(r) => result.add_success(i, r),
                Err(e) => {
                    result.add_failure(i, BatchError::new(e.to_string(), i));
                    if !self.config.continue_on_error {
                        break;
                    }
                }
            }
        }
        result.execution_time = start.elapsed();
        result.total_processed = total;
        result
    }
}
impl Default for BatchExecutor {
    fn default() -> Self {
        Self::new()
    }
}
