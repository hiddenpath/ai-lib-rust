//! 核心反馈类型：提供 FeedbackSink trait 和多种反馈事件（始终编译）。
//!
//! Core feedback types (always compiled).
//!
//! Provides FeedbackSink trait, FeedbackEvent enum, and NoopFeedbackSink for use
//! by the client and other core modules. The full telemetry module (InMemoryFeedbackSink,
//! ConsoleFeedbackSink, etc.) is feature-gated.

use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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

/// No-op sink (always available).
pub struct NoopFeedbackSink;

#[async_trait]
impl FeedbackSink for NoopFeedbackSink {
    async fn report(&self, _: FeedbackEvent) -> Result<()> {
        Ok(())
    }
}

/// Returns a no-op feedback sink.
pub fn noop_sink() -> Arc<dyn FeedbackSink> {
    Arc::new(NoopFeedbackSink)
}
