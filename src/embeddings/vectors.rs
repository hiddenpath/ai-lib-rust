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

#[cfg(test)]
mod tests {
    use super::*;

    const EPSILON: f32 = 1e-6;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < EPSILON
    }

    #[test]
    fn test_dot_product_basic() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let result = dot_product(&a, &b).unwrap();
        // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
        assert!(approx_eq(result, 32.0));
    }

    #[test]
    fn test_dot_product_dimension_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0, 2.0, 3.0];
        assert!(dot_product(&a, &b).is_err());
    }

    #[test]
    fn test_dot_product_orthogonal() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![0.0, 1.0, 0.0];
        let result = dot_product(&a, &b).unwrap();
        assert!(approx_eq(result, 0.0));
    }

    #[test]
    fn test_magnitude_basic() {
        let v = vec![3.0, 4.0];
        let result = magnitude(&v);
        // sqrt(9 + 16) = sqrt(25) = 5
        assert!(approx_eq(result, 5.0));
    }

    #[test]
    fn test_magnitude_unit_vector() {
        let v = vec![1.0, 0.0, 0.0];
        let result = magnitude(&v);
        assert!(approx_eq(result, 1.0));
    }

    #[test]
    fn test_magnitude_zero_vector() {
        let v = vec![0.0, 0.0, 0.0];
        let result = magnitude(&v);
        assert!(approx_eq(result, 0.0));
    }

    #[test]
    fn test_normalize_vector_basic() {
        let v = vec![3.0, 4.0];
        let normalized = normalize_vector(&v);
        // [3/5, 4/5] = [0.6, 0.8]
        assert!(approx_eq(normalized[0], 0.6));
        assert!(approx_eq(normalized[1], 0.8));
        // Magnitude should be 1
        assert!(approx_eq(magnitude(&normalized), 1.0));
    }

    #[test]
    fn test_normalize_vector_zero() {
        let v = vec![0.0, 0.0, 0.0];
        let normalized = normalize_vector(&v);
        // Should return original for zero vector
        assert_eq!(normalized, v);
    }

    #[test]
    fn test_cosine_similarity_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = cosine_similarity(&a, &b).unwrap();
        assert!(approx_eq(result, 1.0));
    }

    #[test]
    fn test_cosine_similarity_opposite() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![-1.0, -2.0, -3.0];
        let result = cosine_similarity(&a, &b).unwrap();
        assert!(approx_eq(result, -1.0));
    }

    #[test]
    fn test_cosine_similarity_orthogonal() {
        let a = vec![1.0, 0.0];
        let b = vec![0.0, 1.0];
        let result = cosine_similarity(&a, &b).unwrap();
        assert!(approx_eq(result, 0.0));
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let a = vec![0.0, 0.0];
        let b = vec![1.0, 1.0];
        let result = cosine_similarity(&a, &b).unwrap();
        // Should return 0 for zero vector
        assert!(approx_eq(result, 0.0));
    }

    #[test]
    fn test_euclidean_distance_basic() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let result = euclidean_distance(&a, &b).unwrap();
        assert!(approx_eq(result, 5.0));
    }

    #[test]
    fn test_euclidean_distance_identical() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = euclidean_distance(&a, &b).unwrap();
        assert!(approx_eq(result, 0.0));
    }

    #[test]
    fn test_manhattan_distance_basic() {
        let a = vec![0.0, 0.0];
        let b = vec![3.0, 4.0];
        let result = manhattan_distance(&a, &b).unwrap();
        // |3-0| + |4-0| = 7
        assert!(approx_eq(result, 7.0));
    }

    #[test]
    fn test_manhattan_distance_negative() {
        let a = vec![1.0, 2.0];
        let b = vec![-1.0, -2.0];
        let result = manhattan_distance(&a, &b).unwrap();
        // |1-(-1)| + |2-(-2)| = 2 + 4 = 6
        assert!(approx_eq(result, 6.0));
    }

    #[test]
    fn test_find_most_similar_cosine() {
        let query = vec![1.0, 0.0, 0.0];
        let candidates = vec![
            vec![1.0, 0.0, 0.0],  // identical
            vec![0.0, 1.0, 0.0],  // orthogonal
            vec![0.7, 0.7, 0.0],  // 45 degrees
        ];
        
        let results = find_most_similar(&query, &candidates, 2, SimilarityMetric::Cosine).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].index, 0); // Identical should be first
        assert!(approx_eq(results[0].score, 1.0));
    }

    #[test]
    fn test_find_most_similar_euclidean() {
        let query = vec![0.0, 0.0];
        let candidates = vec![
            vec![1.0, 0.0],  // distance 1
            vec![3.0, 4.0],  // distance 5
            vec![0.5, 0.5],  // distance ~0.7
        ];
        
        let results = find_most_similar(&query, &candidates, 2, SimilarityMetric::Euclidean).unwrap();
        assert_eq!(results.len(), 2);
        // Euclidean: smaller is better, so closest first
        assert_eq!(results[0].index, 2); // 0.5, 0.5 is closest
    }

    #[test]
    fn test_find_most_similar_top_k() {
        let query = vec![1.0, 0.0];
        let candidates = vec![
            vec![1.0, 0.0],
            vec![0.9, 0.1],
            vec![0.8, 0.2],
            vec![0.0, 1.0],
        ];
        
        let results = find_most_similar(&query, &candidates, 2, SimilarityMetric::Cosine).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_average_vectors_basic() {
        let vectors = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0],
        ];
        let result = average_vectors(&vectors).unwrap();
        // [(1+3)/2, (2+4)/2] = [2, 3]
        assert!(approx_eq(result[0], 2.0));
        assert!(approx_eq(result[1], 3.0));
    }

    #[test]
    fn test_average_vectors_empty() {
        let vectors: Vec<Vec<f32>> = vec![];
        assert!(average_vectors(&vectors).is_err());
    }

    #[test]
    fn test_average_vectors_dimension_mismatch() {
        let vectors = vec![
            vec![1.0, 2.0],
            vec![3.0, 4.0, 5.0],
        ];
        assert!(average_vectors(&vectors).is_err());
    }

    #[test]
    fn test_weighted_average_vectors_basic() {
        let vectors = vec![
            vec![1.0, 0.0],
            vec![0.0, 1.0],
        ];
        let weights = vec![1.0, 1.0];
        let result = weighted_average_vectors(&vectors, &weights).unwrap();
        // Equal weights: [0.5, 0.5]
        assert!(approx_eq(result[0], 0.5));
        assert!(approx_eq(result[1], 0.5));
    }

    #[test]
    fn test_weighted_average_vectors_unequal() {
        let vectors = vec![
            vec![1.0, 0.0],
            vec![0.0, 1.0],
        ];
        let weights = vec![3.0, 1.0]; // 75% first, 25% second
        let result = weighted_average_vectors(&vectors, &weights).unwrap();
        assert!(approx_eq(result[0], 0.75));
        assert!(approx_eq(result[1], 0.25));
    }

    #[test]
    fn test_weighted_average_vectors_zero_weights() {
        let vectors = vec![vec![1.0, 2.0]];
        let weights = vec![0.0];
        assert!(weighted_average_vectors(&vectors, &weights).is_err());
    }

    #[test]
    fn test_add_vectors_basic() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![4.0, 5.0, 6.0];
        let result = add_vectors(&a, &b).unwrap();
        assert_eq!(result, vec![5.0, 7.0, 9.0]);
    }

    #[test]
    fn test_add_vectors_dimension_mismatch() {
        let a = vec![1.0, 2.0];
        let b = vec![1.0];
        assert!(add_vectors(&a, &b).is_err());
    }

    #[test]
    fn test_subtract_vectors_basic() {
        let a = vec![5.0, 7.0, 9.0];
        let b = vec![1.0, 2.0, 3.0];
        let result = subtract_vectors(&a, &b).unwrap();
        assert_eq!(result, vec![4.0, 5.0, 6.0]);
    }

    #[test]
    fn test_scale_vector_basic() {
        let v = vec![1.0, 2.0, 3.0];
        let result = scale_vector(&v, 2.0);
        assert_eq!(result, vec![2.0, 4.0, 6.0]);
    }

    #[test]
    fn test_scale_vector_zero() {
        let v = vec![1.0, 2.0, 3.0];
        let result = scale_vector(&v, 0.0);
        assert_eq!(result, vec![0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_scale_vector_negative() {
        let v = vec![1.0, 2.0];
        let result = scale_vector(&v, -1.0);
        assert_eq!(result, vec![-1.0, -2.0]);
    }
}
