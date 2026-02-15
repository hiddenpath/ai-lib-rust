//! 遥测与反馈模块：可选的应用层遥测和反馈收集（需启用 telemetry 特性）。
//!
//! Telemetry and Feedback Module (feature-gated).
//!
//! This module provides optional, application-controlled telemetry and feedback
//! collection capabilities. Privacy is paramount - the runtime MUST NOT force
//! telemetry collection.
//!
//! ## Overview
//!
//! The feedback system enables:
//! - Collection of user preferences (thumbs up/down, ratings)
//! - Tracking of choice selections in multi-candidate responses
//! - Recording corrections and regeneration requests
//! - Custom feedback integration with external systems
//!
//! ## Key Components
//!
//! | Component | Description |
//! |-----------|-------------|
//! | [`FeedbackEvent`] | Typed feedback event enum |
//! | [`FeedbackSink`] | Trait for feedback destinations |
//! | [`NoopFeedbackSink`] | Default no-op sink (no collection) |
//! | [`InMemoryFeedbackSink`] | In-memory sink for testing |
//! | [`ConsoleFeedbackSink`] | Console logging sink for debugging |
//! | [`CompositeFeedbackSink`] | Multi-destination composite sink |

// Re-export core types from feedback module (always compiled)
pub use crate::feedback::{
    ChoiceSelectionFeedback, CorrectionFeedback, FeedbackEvent, FeedbackSink, NoopFeedbackSink,
    RatingFeedback, RegenerateFeedback, StopFeedback, TextFeedback, ThumbsFeedback, noop_sink,
};

use crate::Result;
use async_trait::async_trait;
use std::sync::{Arc, RwLock};

/// In-memory sink for testing.
pub struct InMemoryFeedbackSink {
    events: Arc<RwLock<Vec<FeedbackEvent>>>,
    max_events: usize,
}
impl InMemoryFeedbackSink {
    pub fn new(max: usize) -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            max_events: max,
        }
    }
    pub fn get_events(&self) -> Vec<FeedbackEvent> {
        self.events.read().unwrap().clone()
    }
    pub fn get_events_by_request(&self, req_id: &str) -> Vec<FeedbackEvent> {
        self.events
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.request_id() == req_id)
            .cloned()
            .collect()
    }
    pub fn clear(&self) {
        self.events.write().unwrap().clear();
    }
    pub fn len(&self) -> usize {
        self.events.read().unwrap().len()
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
#[async_trait]
impl FeedbackSink for InMemoryFeedbackSink {
    async fn report(&self, event: FeedbackEvent) -> Result<()> {
        let mut events = self.events.write().unwrap();
        events.push(event);
        if events.len() > self.max_events {
            events.remove(0);
        }
        Ok(())
    }
}

/// Console sink for debugging.
pub struct ConsoleFeedbackSink {
    prefix: String,
}
impl ConsoleFeedbackSink {
    pub fn new(prefix: impl Into<String>) -> Self {
        Self {
            prefix: prefix.into(),
        }
    }
}
impl Default for ConsoleFeedbackSink {
    fn default() -> Self {
        Self::new("[Feedback]")
    }
}
#[async_trait]
impl FeedbackSink for ConsoleFeedbackSink {
    async fn report(&self, event: FeedbackEvent) -> Result<()> {
        println!("{} {:?}", self.prefix, event);
        Ok(())
    }
}

/// Composite sink for multiple destinations.
pub struct CompositeFeedbackSink {
    sinks: Vec<Arc<dyn FeedbackSink>>,
}
impl CompositeFeedbackSink {
    pub fn new() -> Self {
        Self {
            sinks: Vec::new(),
        }
    }
    pub fn add_sink(mut self, sink: Arc<dyn FeedbackSink>) -> Self {
        self.sinks.push(sink);
        self
    }
}
impl Default for CompositeFeedbackSink {
    fn default() -> Self {
        Self::new()
    }
}
#[async_trait]
impl FeedbackSink for CompositeFeedbackSink {
    async fn report(&self, event: FeedbackEvent) -> Result<()> {
        for s in &self.sinks {
            let _ = s.report(event.clone()).await;
        }
        Ok(())
    }
    async fn close(&self) -> Result<()> {
        for s in &self.sinks {
            let _ = s.close().await;
        }
        Ok(())
    }
}

static GLOBAL_SINK: once_cell::sync::Lazy<RwLock<Arc<dyn FeedbackSink>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(Arc::new(NoopFeedbackSink)));

/// Returns the globally configured feedback sink.
pub fn get_feedback_sink() -> Arc<dyn FeedbackSink> {
    GLOBAL_SINK.read().unwrap().clone()
}

/// Sets the global feedback sink.
pub fn set_feedback_sink(sink: Arc<dyn FeedbackSink>) {
    *GLOBAL_SINK.write().unwrap() = sink;
}

/// Reports feedback to the global sink.
pub async fn report_feedback(event: FeedbackEvent) -> Result<()> {
    get_feedback_sink().report(event).await
}
