//! Vector operations for embeddings.

use crate::{Error, Result};

pub type Vector = Vec<f32>;

pub fn dot_product(a: &[f32], b: &[f32]) -> Result<f32> {
    if a.len() != b.len() {
        return Err(Error::validation(format!("Vector dimensions must match: {} != {}", a.len(), b.len())));
    }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x * y).sum())
}

pub fn magnitude(v: &[f32]) -> f32 {
    v.iter().map(|x| x * x).sum::<f32>().sqrt()
}

pub fn normalize_vector(v: &[f32]) -> Vector {
    let mag = magnitude(v);
    if mag == 0.0 { return v.to_vec(); }
    v.iter().map(|x| x / mag).collect()
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32> {
    if a.len() != b.len() {
        return Err(Error::validation(format!("Vector dimensions must match: {} != {}", a.len(), b.len())));
    }
    let dot = dot_product(a, b)?;
    let mag_a = magnitude(a);
    let mag_b = magnitude(b);
    if mag_a == 0.0 || mag_b == 0.0 { return Ok(0.0); }
    Ok(dot / (mag_a * mag_b))
}

pub fn euclidean_distance(a: &[f32], b: &[f32]) -> Result<f32> {
    if a.len() != b.len() {
        return Err(Error::validation(format!("Vector dimensions must match: {} != {}", a.len(), b.len())));
    }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| (x - y).powi(2)).sum::<f32>().sqrt())
}

pub fn manhattan_distance(a: &[f32], b: &[f32]) -> Result<f32> {
    if a.len() != b.len() {
        return Err(Error::validation(format!("Vector dimensions must match: {} != {}", a.len(), b.len())));
    }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimilarityMetric { Cosine, Euclidean, DotProduct, Manhattan }

#[derive(Debug, Clone)]
pub struct SimilarityResult { pub index: usize, pub score: f32 }

pub fn find_most_similar(query: &[f32], candidates: &[Vec<f32>], top_k: usize, metric: SimilarityMetric) -> Result<Vec<SimilarityResult>> {
    let mut scores: Vec<SimilarityResult> = candidates.iter().enumerate()
        .filter_map(|(i, c)| {
            let score = match metric {
                SimilarityMetric::Cosine => cosine_similarity(query, c).ok(),
                SimilarityMetric::Euclidean => euclidean_distance(query, c).ok(),
                SimilarityMetric::DotProduct => dot_product(query, c).ok(),
                SimilarityMetric::Manhattan => manhattan_distance(query, c).ok(),
            };
            score.map(|s| SimilarityResult { index: i, score: s })
        }).collect();
    match metric {
        SimilarityMetric::Cosine | SimilarityMetric::DotProduct => scores.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)),
        _ => scores.sort_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal)),
    }
    scores.truncate(top_k);
    Ok(scores)
}

pub fn average_vectors(vectors: &[Vec<f32>]) -> Result<Vector> {
    if vectors.is_empty() { return Err(Error::validation("Cannot average empty list")); }
    let dim = vectors[0].len();
    if !vectors.iter().all(|v| v.len() == dim) { return Err(Error::validation("All vectors must have same dimensions")); }
    let n = vectors.len() as f32;
    let mut result = vec![0.0; dim];
    for v in vectors { for (i, val) in v.iter().enumerate() { result[i] += val; } }
    for val in &mut result { *val /= n; }
    Ok(result)
}

pub fn weighted_average_vectors(vectors: &[Vec<f32>], weights: &[f32]) -> Result<Vector> {
    if vectors.is_empty() { return Err(Error::validation("Cannot average empty list")); }
    if vectors.len() != weights.len() { return Err(Error::validation("Vectors and weights must match")); }
    let total: f32 = weights.iter().sum();
    if total == 0.0 { return Err(Error::validation("Total weight cannot be zero")); }
    let dim = vectors[0].len();
    let mut result = vec![0.0; dim];
    for (v, w) in vectors.iter().zip(weights.iter()) {
        let nw = w / total;
        for (i, val) in v.iter().enumerate() { result[i] += val * nw; }
    }
    Ok(result)
}

pub fn add_vectors(a: &[f32], b: &[f32]) -> Result<Vector> {
    if a.len() != b.len() { return Err(Error::validation("Dimensions must match")); }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x + y).collect())
}

pub fn subtract_vectors(a: &[f32], b: &[f32]) -> Result<Vector> {
    if a.len() != b.len() { return Err(Error::validation("Dimensions must match")); }
    Ok(a.iter().zip(b.iter()).map(|(x, y)| x - y).collect())
}

pub fn scale_vector(v: &[f32], scalar: f32) -> Vector {
    v.iter().map(|x| x * scalar).collect()
}
