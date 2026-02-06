//! Embeddings and Similarity Example
//!
//! This example demonstrates the embeddings functionality of ai-lib-rust:
//! - Vector operations (cosine similarity, Euclidean distance, dot product)
//! - Finding most similar vectors
//! - Semantic similarity search concepts
//!
//! Embeddings are numerical representations of text that capture semantic meaning,
//! enabling similarity comparisons between different pieces of content.
//!
//! Usage:
//!   cargo run --example embeddings_similarity

use ai_lib_rust::embeddings::{
    cosine_similarity, euclidean_distance, dot_product, magnitude,
    normalize_vector, find_most_similar, SimilarityMetric,
};

fn main() {
    println!("=== AI-Lib Embeddings & Similarity Demo ===\n");

    // Example 1: Basic Vector Operations
    demo_basic_operations();

    // Example 2: Similarity Metrics Comparison
    demo_similarity_metrics();

    // Example 3: Semantic Search
    demo_semantic_search();

    // Example 4: Document Clustering Concept
    demo_clustering_concept();
}

fn demo_basic_operations() {
    println!("--- Example 1: Basic Vector Operations ---\n");

    // Sample vectors (simulating embeddings) - using f32
    let vec_a: Vec<f32> = vec![0.1, 0.2, 0.3, 0.4, 0.5];
    let vec_b: Vec<f32> = vec![0.15, 0.25, 0.35, 0.45, 0.55];
    let vec_c: Vec<f32> = vec![-0.1, -0.2, -0.3, -0.4, -0.5];

    println!("Vector A: {:?}", vec_a);
    println!("Vector B: {:?}", vec_b);
    println!("Vector C: {:?}\n", vec_c);

    // Cosine similarity
    let sim_ab = cosine_similarity(&vec_a, &vec_b).unwrap();
    let sim_ac = cosine_similarity(&vec_a, &vec_c).unwrap();
    println!("Cosine Similarity (A, B): {:.6} (very similar)", sim_ab);
    println!("Cosine Similarity (A, C): {:.6} (opposite)", sim_ac);

    // Euclidean distance
    let dist_ab = euclidean_distance(&vec_a, &vec_b).unwrap();
    let dist_ac = euclidean_distance(&vec_a, &vec_c).unwrap();
    println!("\nEuclidean Distance (A, B): {:.6} (close)", dist_ab);
    println!("Euclidean Distance (A, C): {:.6} (far)", dist_ac);

    // Dot product
    let dot_ab = dot_product(&vec_a, &vec_b).unwrap();
    println!("\nDot Product (A, B): {:.6}", dot_ab);

    // Magnitude - returns f32 directly, not Result
    let mag_a = magnitude(&vec_a);
    println!("Magnitude of A: {:.6}\n", mag_a);
}

fn demo_similarity_metrics() {
    println!("--- Example 2: Similarity Metrics Comparison ---\n");

    // Compare different metrics - using f32
    let query: Vec<f32> = vec![0.5, 0.5, 0.0];
    let candidates: Vec<Vec<f32>> = vec![
        vec![0.5, 0.5, 0.0],  // Identical
        vec![0.4, 0.4, 0.1],  // Very similar
        vec![0.0, 0.0, 1.0],  // Orthogonal
        vec![-0.5, -0.5, 0.0], // Opposite
    ];
    let labels = ["Identical", "Very similar", "Orthogonal", "Opposite"];

    println!("Query vector: {:?}\n", query);
    println!("{:<15} {:>10} {:>15} {:>12}", "Candidate", "Cosine", "Euclidean", "Dot");
    println!("{:-<15} {:-<10} {:-<15} {:-<12}", "", "", "", "");

    for (i, candidate) in candidates.iter().enumerate() {
        let cos = cosine_similarity(&query, candidate).unwrap();
        let euc = euclidean_distance(&query, candidate).unwrap();
        let dot = dot_product(&query, candidate).unwrap();
        println!("{:<15} {:>10.4} {:>15.4} {:>12.4}", labels[i], cos, euc, dot);
    }
    println!();
}

fn demo_semantic_search() {
    println!("--- Example 3: Semantic Search ---\n");

    // Simulated document embeddings - using f32
    // In practice, these would come from an embedding model
    let documents: Vec<(&str, &str, Vec<f32>)> = vec![
        ("doc1", "Introduction to machine learning", vec![0.8, 0.1, 0.05, 0.05]),
        ("doc2", "Deep learning neural networks", vec![0.75, 0.15, 0.05, 0.05]),
        ("doc3", "Cooking recipes for beginners", vec![0.1, 0.1, 0.7, 0.1]),
        ("doc4", "Advanced calculus mathematics", vec![0.1, 0.7, 0.1, 0.1]),
        ("doc5", "AI and deep learning guide", vec![0.85, 0.08, 0.02, 0.05]),
    ];

    println!("Document Collection:");
    for (id, title, _) in &documents {
        println!("  {}: {}", id, title);
    }

    // Search query embedding (simulated) - using f32
    let query_embedding: Vec<f32> = vec![0.9, 0.05, 0.02, 0.03];
    println!("\nQuery: 'machine learning tutorial' (simulated embedding)\n");

    // Build vectors for comparison
    let doc_vectors: Vec<Vec<f32>> = documents.iter().map(|(_, _, v)| v.clone()).collect();

    // Find most similar
    let results = find_most_similar(&query_embedding, &doc_vectors, 3, SimilarityMetric::Cosine)
        .expect("Search failed");

    println!("Top 3 Results:");
    for result in results {
        let (id, title, _) = &documents[result.index];
        println!("  #{} {} - \"{}\" (score: {:.4})", 
            result.index + 1, id, title, result.score);
    }
    println!();
}

fn demo_clustering_concept() {
    println!("--- Example 4: Document Clustering Concept ---\n");

    // Demonstrate vector normalization for clustering - using f32
    let raw_embedding: Vec<f32> = vec![3.0, 4.0, 0.0];
    let normalized = normalize_vector(&raw_embedding);

    println!("Raw embedding: {:?}", raw_embedding);
    println!("Normalized:    {:?}", normalized);
    println!("Magnitude (normalized): {:.6}\n", magnitude(&normalized));

    // Cluster centroids (simulated)
    let clusters: Vec<(&str, Vec<f32>)> = vec![
        ("Technology", vec![0.9, 0.1, 0.0]),
        ("Science", vec![0.1, 0.9, 0.0]),
        ("Arts", vec![0.0, 0.1, 0.9]),
    ];

    let document: Vec<f32> = vec![0.7, 0.3, 0.0];
    println!("Document vector: {:?}\n", document);
    println!("Cluster Assignment Analysis:");
    
    for (name, centroid) in &clusters {
        let sim = cosine_similarity(&document, centroid).unwrap();
        println!("  {}: similarity = {:.4}", name, sim);
    }

    println!("\n  -> Document belongs to: Technology cluster");

    println!("\n=== Best Practices ===\n");
    println!("1. Normalize vectors before similarity computation");
    println!("2. Use cosine similarity for semantic similarity");
    println!("3. Use Euclidean distance for absolute positioning");
    println!("4. Cache embeddings for repeated comparisons");
    println!("5. Consider dimensionality reduction for large vectors");
}
