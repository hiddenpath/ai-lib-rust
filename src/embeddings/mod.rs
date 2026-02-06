//! 向量嵌入模块：提供文本嵌入生成和向量相似度计算功能。
//!
//! # Embeddings Module
//!
//! This module provides comprehensive embedding support for AI models, enabling
//! semantic similarity comparisons and vector-based operations.
//!
//! ## Overview
//!
//! Embeddings are numerical vector representations of text that capture semantic
//! meaning, enabling:
//! - Semantic search and similarity matching
//! - Document clustering and classification
//! - Recommendation systems
//! - Duplicate detection
//!
//! ## Key Features
//!
//! | Feature | Description |
//! |---------|-------------|
//! | **Embedding generation** | Generate embeddings via AI providers |
//! | **Vector similarity** | Cosine, Euclidean, Manhattan, Dot product metrics |
//! | **Vector operations** | Normalize, average, add, subtract, scale vectors |
//! | **Similarity search** | Find most similar vectors in a collection |
//!
//! ## Components
//!
//! | Component | Description |
//! |-----------|-------------|
//! | [`EmbeddingClient`] | Client for generating embeddings from AI providers |
//! | [`cosine_similarity`] | Cosine similarity between vectors (-1 to 1) |
//! | [`euclidean_distance`] | Euclidean (L2) distance between vectors |
//! | [`find_most_similar`] | Find top-k most similar vectors |
//! | [`normalize_vector`] | Normalize vector to unit length |
//!
//! ## Example
//!
//! ```rust
//! use ai_lib_rust::embeddings::{cosine_similarity, find_most_similar, SimilarityMetric};
//!
//! // Calculate similarity between two vectors
//! let vec_a: Vec<f32> = vec![0.1, 0.2, 0.3];
//! let vec_b: Vec<f32> = vec![0.15, 0.25, 0.35];
//! let similarity = cosine_similarity(&vec_a, &vec_b).unwrap();
//! println!("Similarity: {:.4}", similarity);
//!
//! // Find most similar vectors in a collection
//! let query: Vec<f32> = vec![0.5, 0.5, 0.0];
//! let candidates: Vec<Vec<f32>> = vec![
//!     vec![0.5, 0.5, 0.0],
//!     vec![0.0, 0.0, 1.0],
//! ];
//! let results = find_most_similar(&query, &candidates, 1, SimilarityMetric::Cosine).unwrap();
//! ```
//!
//! ## Metrics Comparison
//!
//! | Metric | Range | Best For |
//! |--------|-------|----------|
//! | Cosine | -1 to 1 | Semantic similarity (direction) |
//! | Euclidean | 0 to ∞ | Absolute positioning |
//! | Dot Product | -∞ to ∞ | Magnitude-sensitive comparison |
//! | Manhattan | 0 to ∞ | Sparse vectors, grid distances |

mod client;
mod types;
mod vectors;

pub use client::{EmbeddingClient, EmbeddingClientBuilder};
pub use types::{
    Embedding, EmbeddingInput, EmbeddingModel, EmbeddingRequest, EmbeddingResponse, EmbeddingUsage,
};
pub use vectors::{
    add_vectors, average_vectors, cosine_similarity, dot_product, euclidean_distance,
    find_most_similar, magnitude, manhattan_distance, normalize_vector, scale_vector,
    subtract_vectors, weighted_average_vectors, SimilarityMetric, SimilarityResult, Vector,
};
