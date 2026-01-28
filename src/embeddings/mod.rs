//! Embedding support for AI models.
//!
//! This module provides:
//! - Embedding client for generating embeddings
//! - Vector operations (similarity, distance, normalization)
//! - Types for embedding requests and responses

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
