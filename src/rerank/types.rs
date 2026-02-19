//! Rerank types.

use serde::{Deserialize, Serialize};

/// A single rerank result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RerankResult {
    pub index: usize,
    pub relevance_score: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<String>,
}

/// Options for reranking.
#[derive(Debug, Clone, Default)]
pub struct RerankOptions {
    pub top_n: Option<usize>,
    pub max_tokens_per_doc: Option<usize>,
}
