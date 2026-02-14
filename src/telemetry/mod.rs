//! 遥测与反馈模块：提供可选的、应用可控的用户反馈收集机制。
//!
//! # Telemetry and Feedback Module
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
//! ## Design Principles
//!
//! - **Opt-in Only**: No telemetry is collected unless explicitly configured
//! - **Application-Controlled**: The application decides what to collect and where to send
//! - **Stable Linkage**: `client_request_id` provides correlation across events
//! - **Pluggable Sinks**: Implement [`FeedbackSink`] for custom destinations
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
//!
//! ## Feedback Types
//!
//! | Type | Description |
//! |------|-------------|
//! | [`ThumbsFeedback`] | Simple positive/negative feedback |
//! | [`RatingFeedback`] | Numeric rating (e.g., 1-5 stars) |
//! | [`ChoiceSelectionFeedback`] | Multi-candidate selection tracking |
//! | [`CorrectionFeedback`] | User corrections to model output |
//! | [`RegenerateFeedback`] | Regeneration request tracking |
//! | [`TextFeedback`] | Free-form text feedback |
//!
//! ## Example
//!
//! ```rust
//! use ai_lib_rust::telemetry::{FeedbackEvent, ThumbsFeedback, set_feedback_sink, InMemoryFeedbackSink};
//! use std::sync::Arc;
//!
//! // Configure feedback collection (opt-in)
//! let sink = Arc::new(InMemoryFeedbackSink::new(100));
//! set_feedback_sink(sink.clone());
//!
//! // Record feedback
//! let feedback = ThumbsFeedback::thumbs_up("req-123")
//!     .with_reason("Helpful response");
//! // report_feedback(FeedbackEvent::Thumbs(feedback)).await?;
//! ```

use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

fn timestamp() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}

/// Feedback for multi-candidate selection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoiceSelectionFeedback {
    pub request_id: String,
    pub chosen_index: u32,
    pub rejected_indices: Option<Vec<u32>>,
    pub latency_to_select_ms: Option<u64>,
    pub ui_context: Option<serde_json::Value>,
    pub candidate_hashes: Option<Vec<String>>,
    pub timestamp: f64,
}

impl ChoiceSelectionFeedback {
    pub fn new(request_id: impl Into<String>, chosen_index: u32) -> Self {
        Self {
            request_id: request_id.into(),
            chosen_index,
            rejected_indices: None,
            latency_to_select_ms: None,
            ui_context: None,
            candidate_hashes: None,
            timestamp: timestamp(),
        }
    }
    pub fn with_rejected(mut self, indices: Vec<u32>) -> Self {
        self.rejected_indices = Some(indices);
        self
    }
    pub fn with_latency(mut self, ms: u64) -> Self {
        self.latency_to_select_ms = Some(ms);
        self
    }
}

/// Rating feedback (e.g., 1-5 stars).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RatingFeedback {
    pub request_id: String,
    pub rating: u32,
    pub max_rating: u32,
    pub category: Option<String>,
    pub comment: Option<String>,
    pub timestamp: f64,
}
impl RatingFeedback {
    pub fn new(request_id: impl Into<String>, rating: u32) -> Self {
        Self {
            request_id: request_id.into(),
            rating,
            max_rating: 5,
            category: None,
            comment: None,
            timestamp: timestamp(),
        }
    }
    pub fn with_max_rating(mut self, m: u32) -> Self {
        self.max_rating = m;
        self
    }
    pub fn with_comment(mut self, c: impl Into<String>) -> Self {
        self.comment = Some(c.into());
        self
    }
}

/// Thumbs up/down feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThumbsFeedback {
    pub request_id: String,
    pub is_positive: bool,
    pub reason: Option<String>,
    pub timestamp: f64,
}
impl ThumbsFeedback {
    pub fn thumbs_up(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            is_positive: true,
            reason: None,
            timestamp: timestamp(),
        }
    }
    pub fn thumbs_down(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            is_positive: false,
            reason: None,
            timestamp: timestamp(),
        }
    }
    pub fn with_reason(mut self, r: impl Into<String>) -> Self {
        self.reason = Some(r.into());
        self
    }
}

/// Free-form text feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextFeedback {
    pub request_id: String,
    pub text: String,
    pub category: Option<String>,
    pub timestamp: f64,
}
impl TextFeedback {
    pub fn new(request_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            text: text.into(),
            category: None,
            timestamp: timestamp(),
        }
    }
}

/// Correction feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrectionFeedback {
    pub request_id: String,
    pub original_hash: String,
    pub corrected_hash: String,
    pub edit_distance: Option<u32>,
    pub correction_type: Option<String>,
    pub timestamp: f64,
}
impl CorrectionFeedback {
    pub fn new(
        request_id: impl Into<String>,
        original: impl Into<String>,
        corrected: impl Into<String>,
    ) -> Self {
        Self {
            request_id: request_id.into(),
            original_hash: original.into(),
            corrected_hash: corrected.into(),
            edit_distance: None,
            correction_type: None,
            timestamp: timestamp(),
        }
    }
}

/// Regeneration feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegenerateFeedback {
    pub request_id: String,
    pub regeneration_count: u32,
    pub reason: Option<String>,
    pub timestamp: f64,
}
impl RegenerateFeedback {
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            regeneration_count: 1,
            reason: None,
            timestamp: timestamp(),
        }
    }
}

/// Stop generation feedback.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopFeedback {
    pub request_id: String,
    pub tokens_generated: Option<u32>,
    pub reason: Option<String>,
    pub timestamp: f64,
}
impl StopFeedback {
    pub fn new(request_id: impl Into<String>) -> Self {
        Self {
            request_id: request_id.into(),
            tokens_generated: None,
            reason: None,
            timestamp: timestamp(),
        }
    }
}

/// Typed feedback events (extensible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FeedbackEvent {
    ChoiceSelection(ChoiceSelectionFeedback),
    Rating(RatingFeedback),
    Thumbs(ThumbsFeedback),
    Text(TextFeedback),
    Correction(CorrectionFeedback),
    Regenerate(RegenerateFeedback),
    Stop(StopFeedback),
}

impl FeedbackEvent {
    pub fn request_id(&self) -> &str {
        match self {
            FeedbackEvent::ChoiceSelection(f) => &f.request_id,
            FeedbackEvent::Rating(f) => &f.request_id,
            FeedbackEvent::Thumbs(f) => &f.request_id,
            FeedbackEvent::Text(f) => &f.request_id,
            FeedbackEvent::Correction(f) => &f.request_id,
            FeedbackEvent::Regenerate(f) => &f.request_id,
            FeedbackEvent::Stop(f) => &f.request_id,
        }
    }
}

/// Feedback sink trait.
#[async_trait]
pub trait FeedbackSink: Send + Sync {
    async fn report(&self, event: FeedbackEvent) -> Result<()>;
    async fn report_batch(&self, events: Vec<FeedbackEvent>) -> Result<()> {
        for e in events {
            self.report(e).await?;
        }
        Ok(())
    }
    async fn close(&self) -> Result<()> {
        Ok(())
    }
}

/// No-op sink.
pub struct NoopFeedbackSink;
#[async_trait]
impl FeedbackSink for NoopFeedbackSink {
    async fn report(&self, _: FeedbackEvent) -> Result<()> {
        Ok(())
    }
}

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
        Self { sinks: Vec::new() }
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

pub fn noop_sink() -> Arc<dyn FeedbackSink> {
    Arc::new(NoopFeedbackSink)
}

static GLOBAL_SINK: once_cell::sync::Lazy<RwLock<Arc<dyn FeedbackSink>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(Arc::new(NoopFeedbackSink)));
pub fn get_feedback_sink() -> Arc<dyn FeedbackSink> {
    GLOBAL_SINK.read().unwrap().clone()
}
pub fn set_feedback_sink(sink: Arc<dyn FeedbackSink>) {
    *GLOBAL_SINK.write().unwrap() = sink;
}
pub async fn report_feedback(event: FeedbackEvent) -> Result<()> {
    get_feedback_sink().report(event).await
}
