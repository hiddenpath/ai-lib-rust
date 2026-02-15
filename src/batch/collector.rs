//! Batch collector.

use std::collections::VecDeque;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct BatchConfig {
    pub max_batch_size: usize,
    pub max_wait_time: Duration,
    pub auto_flush: bool,
}
impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 10,
            max_wait_time: Duration::from_secs(5),
            auto_flush: true,
        }
    }
}
impl BatchConfig {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn with_max_batch_size(mut self, s: usize) -> Self {
        self.max_batch_size = s;
        self
    }
    pub fn with_auto_flush(mut self, a: bool) -> Self {
        self.auto_flush = a;
        self
    }
}

#[derive(Debug, Clone)]
pub struct BatchItem<T> {
    pub data: T,
    pub added_at: Instant,
    pub request_id: Option<String>,
    pub priority: i32,
}
impl<T> BatchItem<T> {
    pub fn new(data: T) -> Self {
        Self {
            data,
            added_at: Instant::now(),
            request_id: None,
            priority: 0,
        }
    }
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
    pub fn with_priority(mut self, p: i32) -> Self {
        self.priority = p;
        self
    }
}

pub struct BatchCollector<T> {
    config: BatchConfig,
    items: Arc<RwLock<VecDeque<BatchItem<T>>>>,
    batch_start: Arc<RwLock<Option<Instant>>>,
}

impl<T: Clone> BatchCollector<T> {
    pub fn new(config: BatchConfig) -> Self {
        Self {
            config,
            items: Arc::new(RwLock::new(VecDeque::new())),
            batch_start: Arc::new(RwLock::new(None)),
        }
    }

    pub fn add(&self, item: BatchItem<T>) -> BatchAddResult {
        let mut items = self.items.write().unwrap();
        let mut start = self.batch_start.write().unwrap();
        if items.is_empty() {
            *start = Some(Instant::now());
        }
        items.push_back(item);
        let count = items.len();
        if self.config.auto_flush && count >= self.config.max_batch_size {
            BatchAddResult::ShouldFlush { count }
        } else {
            BatchAddResult::Added { count }
        }
    }

    pub fn add_data(&self, data: T) -> BatchAddResult {
        self.add(BatchItem::new(data))
    }

    pub fn should_flush(&self) -> bool {
        let items = self.items.read().unwrap();
        let start = self.batch_start.read().unwrap();
        if items.is_empty() {
            return false;
        }
        if items.len() >= self.config.max_batch_size {
            return true;
        }
        if let Some(s) = *start {
            if s.elapsed() >= self.config.max_wait_time {
                return true;
            }
        }
        false
    }

    pub fn drain(&self) -> Vec<BatchItem<T>> {
        let mut items = self.items.write().unwrap();
        let mut start = self.batch_start.write().unwrap();
        *start = None;
        items.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.items.read().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn clear(&self) {
        self.items.write().unwrap().clear();
        *self.batch_start.write().unwrap() = None;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchAddResult { Added { count: usize }, ShouldFlush { count: usize } }
impl BatchAddResult { pub fn should_flush(&self) -> bool { matches!(self, BatchAddResult::ShouldFlush { .. }) } pub fn count(&self) -> usize { match self { BatchAddResult::Added { count } | BatchAddResult::ShouldFlush { count } => *count } } }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_config_defaults() {
        let config = BatchConfig::default();
        assert_eq!(config.max_batch_size, 10);
        assert_eq!(config.max_wait_time, Duration::from_secs(5));
        assert!(config.auto_flush);
    }

    #[test]
    fn test_batch_config_builder() {
        let config = BatchConfig::new()
            .with_max_batch_size(5)
            .with_auto_flush(false);
        assert_eq!(config.max_batch_size, 5);
        assert!(!config.auto_flush);
    }

    #[test]
    fn test_batch_item_creation() {
        let item = BatchItem::new("test data")
            .with_request_id("req-001")
            .with_priority(10);
        assert_eq!(item.data, "test data");
        assert_eq!(item.request_id, Some("req-001".to_string()));
        assert_eq!(item.priority, 10);
    }

    #[test]
    fn test_batch_collector_empty() {
        let config = BatchConfig::new().with_max_batch_size(5);
        let collector: BatchCollector<String> = BatchCollector::new(config);
        assert!(collector.is_empty());
        assert_eq!(collector.len(), 0);
        assert!(!collector.should_flush());
    }

    #[test]
    fn test_batch_collector_add_data() {
        let config = BatchConfig::new().with_max_batch_size(5);
        let collector: BatchCollector<String> = BatchCollector::new(config);
        
        let result = collector.add_data("item1".to_string());
        assert_eq!(result, BatchAddResult::Added { count: 1 });
        assert_eq!(collector.len(), 1);
        assert!(!collector.is_empty());
    }

    #[test]
    fn test_batch_collector_add_item() {
        let config = BatchConfig::new().with_max_batch_size(5);
        let collector: BatchCollector<String> = BatchCollector::new(config);
        
        let item = BatchItem::new("item1".to_string()).with_priority(5);
        let result = collector.add(item);
        assert_eq!(result.count(), 1);
    }

    #[test]
    fn test_batch_collector_auto_flush() {
        let config = BatchConfig::new().with_max_batch_size(3).with_auto_flush(true);
        let collector: BatchCollector<i32> = BatchCollector::new(config);
        
        // Add items below threshold
        assert!(!collector.add_data(1).should_flush());
        assert!(!collector.add_data(2).should_flush());
        
        // Third item should trigger flush
        let result = collector.add_data(3);
        assert!(result.should_flush());
        assert_eq!(result.count(), 3);
    }

    #[test]
    fn test_batch_collector_no_auto_flush() {
        let config = BatchConfig::new().with_max_batch_size(3).with_auto_flush(false);
        let collector: BatchCollector<i32> = BatchCollector::new(config);
        
        collector.add_data(1);
        collector.add_data(2);
        let result = collector.add_data(3);
        
        // Should not report ShouldFlush when auto_flush is disabled
        assert!(!result.should_flush());
        // But should_flush() method checks size
        assert!(collector.should_flush());
    }

    #[test]
    fn test_batch_collector_drain() {
        let config = BatchConfig::new().with_max_batch_size(10);
        let collector: BatchCollector<String> = BatchCollector::new(config);
        
        collector.add_data("a".to_string());
        collector.add_data("b".to_string());
        collector.add_data("c".to_string());
        
        let items = collector.drain();
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].data, "a");
        assert_eq!(items[1].data, "b");
        assert_eq!(items[2].data, "c");
        
        // Collector should be empty after drain
        assert!(collector.is_empty());
    }

    #[test]
    fn test_batch_collector_clear() {
        let config = BatchConfig::new().with_max_batch_size(10);
        let collector: BatchCollector<i32> = BatchCollector::new(config);
        
        collector.add_data(1);
        collector.add_data(2);
        assert_eq!(collector.len(), 2);
        
        collector.clear();
        assert!(collector.is_empty());
    }

    #[test]
    fn test_batch_add_result_methods() {
        let added = BatchAddResult::Added { count: 5 };
        let should_flush = BatchAddResult::ShouldFlush { count: 10 };
        
        assert!(!added.should_flush());
        assert_eq!(added.count(), 5);
        
        assert!(should_flush.should_flush());
        assert_eq!(should_flush.count(), 10);
    }

    #[test]
    fn test_batch_collector_thread_safe() {
        use std::sync::Arc;
        use std::thread;
        
        let config = BatchConfig::new().with_max_batch_size(100);
        let collector: Arc<BatchCollector<i32>> = Arc::new(BatchCollector::new(config));
        
        let mut handles = vec![];
        for i in 0..10 {
            let c = Arc::clone(&collector);
            handles.push(thread::spawn(move || {
                for j in 0..10 {
                    c.add_data(i * 10 + j);
                }
            }));
        }
        
        for h in handles {
            h.join().unwrap();
        }
        
        assert_eq!(collector.len(), 100);
    }
}
