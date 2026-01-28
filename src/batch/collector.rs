//! Batch collector.

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BatchConfig { pub max_batch_size: usize, pub max_wait_time: Duration, pub auto_flush: bool }
impl Default for BatchConfig { fn default() -> Self { Self { max_batch_size: 10, max_wait_time: Duration::from_secs(5), auto_flush: true } } }
impl BatchConfig {
    pub fn new() -> Self { Self::default() }
    pub fn with_max_batch_size(mut self, s: usize) -> Self { self.max_batch_size = s; self }
    pub fn with_auto_flush(mut self, a: bool) -> Self { self.auto_flush = a; self }
}

#[derive(Debug, Clone)]
pub struct BatchItem<T> { pub data: T, pub added_at: Instant, pub request_id: Option<String>, pub priority: i32 }
impl<T> BatchItem<T> {
    pub fn new(data: T) -> Self { Self { data, added_at: Instant::now(), request_id: None, priority: 0 } }
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self { self.request_id = Some(id.into()); self }
    pub fn with_priority(mut self, p: i32) -> Self { self.priority = p; self }
}

pub struct BatchCollector<T> { config: BatchConfig, items: Arc<RwLock<VecDeque<BatchItem<T>>>>, batch_start: Arc<RwLock<Option<Instant>>> }

impl<T: Clone> BatchCollector<T> {
    pub fn new(config: BatchConfig) -> Self { Self { config, items: Arc::new(RwLock::new(VecDeque::new())), batch_start: Arc::new(RwLock::new(None)) } }

    pub fn add(&self, item: BatchItem<T>) -> BatchAddResult {
        let mut items = self.items.write().unwrap();
        let mut start = self.batch_start.write().unwrap();
        if items.is_empty() { *start = Some(Instant::now()); }
        items.push_back(item);
        let count = items.len();
        if self.config.auto_flush && count >= self.config.max_batch_size { BatchAddResult::ShouldFlush { count } } else { BatchAddResult::Added { count } }
    }

    pub fn add_data(&self, data: T) -> BatchAddResult { self.add(BatchItem::new(data)) }

    pub fn should_flush(&self) -> bool {
        let items = self.items.read().unwrap();
        let start = self.batch_start.read().unwrap();
        if items.is_empty() { return false; }
        if items.len() >= self.config.max_batch_size { return true; }
        if let Some(s) = *start { if s.elapsed() >= self.config.max_wait_time { return true; } }
        false
    }

    pub fn drain(&self) -> Vec<BatchItem<T>> {
        let mut items = self.items.write().unwrap();
        let mut start = self.batch_start.write().unwrap();
        *start = None;
        items.drain(..).collect()
    }

    pub fn len(&self) -> usize { self.items.read().unwrap().len() }
    pub fn is_empty(&self) -> bool { self.len() == 0 }
    pub fn clear(&self) { self.items.write().unwrap().clear(); *self.batch_start.write().unwrap() = None; }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchAddResult { Added { count: usize }, ShouldFlush { count: usize } }
impl BatchAddResult { pub fn should_flush(&self) -> bool { matches!(self, BatchAddResult::ShouldFlush { .. }) } pub fn count(&self) -> usize { match self { BatchAddResult::Added { count } | BatchAddResult::ShouldFlush { count } => *count } } }
