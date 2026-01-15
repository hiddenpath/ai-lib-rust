//! Telemetry and feedback (optional, application-controlled).
//!
//! The runtime MUST NOT force telemetry collection. Instead it provides:
//! - a stable `client_request_id` for linkage
//! - typed feedback events
//! - an injectable `FeedbackSink` hook (default: no-op)

use crate::Result;
use async_trait::async_trait;
use std::sync::Arc;

/// Minimal user feedback for multi-candidate selection.
#[derive(Debug, Clone)]
pub struct ChoiceSelectionFeedback {
    /// Request identifier emitted by the runtime (`client_request_id`).
    pub request_id: String,
    /// The chosen candidate index (0-based).
    pub chosen_index: u32,
    /// Optional rejected indices (0-based).
    pub rejected_indices: Option<Vec<u32>>,
    /// Time from render to selection (ms), if the UI can measure it.
    pub latency_to_select_ms: Option<u64>,
    /// Optional UI context (component name / experiment id / etc.)
    pub ui_context: Option<serde_json::Value>,
    /// Optional content hashes to link choice to rendered candidates without uploading text.
    pub candidate_hashes: Option<Vec<String>>,
}

/// Typed feedback events (extensible).
#[derive(Debug, Clone)]
pub enum FeedbackEvent {
    ChoiceSelection(ChoiceSelectionFeedback),
}

/// Feedback sink hook. Applications decide whether and where to store/report feedback.
#[async_trait]
pub trait FeedbackSink: Send + Sync {
    async fn report(&self, event: FeedbackEvent) -> Result<()>;
}

/// Default sink: do nothing.
pub struct NoopFeedbackSink;

#[async_trait]
impl FeedbackSink for NoopFeedbackSink {
    async fn report(&self, _event: FeedbackEvent) -> Result<()> {
        Ok(())
    }
}

pub fn noop_sink() -> Arc<dyn FeedbackSink> {
    Arc::new(NoopFeedbackSink)
}
