//! Token counter implementations.

use crate::types::message::{ContentBlock, MessageContent};
use crate::types::Message;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub trait TokenCounter: Send + Sync {
    fn count(&self, text: &str) -> usize;

    fn count_messages(&self, messages: &[Message]) -> usize {
        let mut total = 0;
        for message in messages {
            total += 1;
            match &message.content {
                MessageContent::Text(text) => {
                    total += self.count(text);
                }
                MessageContent::Blocks(blocks) => {
                    for block in blocks {
                        match block {
                            ContentBlock::Text { text } => {
                                total += self.count(text);
                            }
                            ContentBlock::Image { .. } => {
                                total += 85;
                            }
                            ContentBlock::Audio { .. } => {
                                total += 100;
                            }
                            ContentBlock::ToolUse { input, .. } => {
                                total +=
                                    self.count(&serde_json::to_string(input).unwrap_or_default());
                            }
                            ContentBlock::ToolResult { content, .. } => {
                                total +=
                                    self.count(&serde_json::to_string(content).unwrap_or_default());
                            }
                        }
                    }
                }
            }
        }
        total + messages.len() * 3
    }

    fn truncate_to_limit(&self, text: &str, max_tokens: usize, suffix: &str) -> String {
        let current = self.count(text);
        if current <= max_tokens {
            return text.to_string();
        }
        let suffix_tokens = if suffix.is_empty() {
            0
        } else {
            self.count(suffix)
        };
        let target = max_tokens.saturating_sub(suffix_tokens);
        if target == 0 {
            return suffix.to_string();
        }
        let chars_per_token = text.len() as f64 / current as f64;
        let mut truncated: String = text
            .chars()
            .take((target as f64 * chars_per_token) as usize)
            .collect();
        while self.count(&truncated) > target && !truncated.is_empty() {
            truncated = truncated
                .chars()
                .take((truncated.len() as f64 * 0.9) as usize)
                .collect();
        }
        format!("{}{}", truncated, suffix)
    }
}

#[derive(Debug, Clone)]
pub struct CharacterEstimator {
    chars_per_token: f64,
}
impl CharacterEstimator {
    pub fn new() -> Self {
        Self::with_ratio(4.0)
    }
    pub fn with_ratio(r: f64) -> Self {
        Self { chars_per_token: r }
    }
}
impl Default for CharacterEstimator {
    fn default() -> Self {
        Self::new()
    }
}
impl TokenCounter for CharacterEstimator {
    fn count(&self, text: &str) -> usize {
        (text.len() as f64 / self.chars_per_token).ceil() as usize
    }
}

#[derive(Debug, Clone)]
pub struct AnthropicEstimator {
    chars_per_token: f64,
}
impl AnthropicEstimator {
    pub fn new() -> Self {
        Self {
            chars_per_token: 3.5,
        }
    }
}
impl Default for AnthropicEstimator {
    fn default() -> Self {
        Self::new()
    }
}
impl TokenCounter for AnthropicEstimator {
    fn count(&self, text: &str) -> usize {
        let base = (text.len() as f64 / self.chars_per_token).ceil() as usize;
        let ws = text.chars().filter(|c| c.is_whitespace()).count();
        base + (ws as f64 * 0.1) as usize
    }
}

pub struct CachingCounter {
    inner: Box<dyn TokenCounter>,
    cache: Arc<RwLock<HashMap<String, usize>>>,
    max_size: usize,
}
impl CachingCounter {
    pub fn new(inner: Box<dyn TokenCounter>, max_size: usize) -> Self {
        Self {
            inner,
            cache: Arc::new(RwLock::new(HashMap::new())),
            max_size,
        }
    }
    pub fn clear_cache(&self) {
        self.cache.write().unwrap().clear();
    }
}
impl TokenCounter for CachingCounter {
    fn count(&self, text: &str) -> usize {
        {
            let c = self.cache.read().unwrap();
            if let Some(&n) = c.get(text) {
                return n;
            }
        }
        let n = self.inner.count(text);
        {
            let mut c = self.cache.write().unwrap();
            if c.len() < self.max_size {
                c.insert(text.to_string(), n);
            }
        }
        n
    }
}

static COUNTERS: once_cell::sync::Lazy<RwLock<HashMap<String, Arc<dyn TokenCounter>>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

pub fn get_token_counter(model: &str) -> Arc<dyn TokenCounter> {
    let ml = model.to_lowercase();
    {
        let c = COUNTERS.read().unwrap();
        if let Some(x) = c.get(&ml) {
            return x.clone();
        }
    }
    let counter: Arc<dyn TokenCounter> = if ml.contains("gpt") || ml.contains("o1") {
        Arc::new(CharacterEstimator::new())
    } else if ml.contains("claude") {
        Arc::new(AnthropicEstimator::new())
    } else {
        Arc::new(CharacterEstimator::new())
    };
    {
        let mut c = COUNTERS.write().unwrap();
        c.insert(ml, counter.clone());
    }
    counter
}
