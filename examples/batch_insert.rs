//! Example: Batch Insert API
//!
//! Demonstrates the batch insertion API for efficiently inserting
//! multiple vectors into an HNSW index.
//!
//! Run with: `cargo run --example batch_insert`

use edgevec::batch::BatchInsertable;
use edgevec::error::BatchError;
use edgevec::{HnswConfig, HnswIndex, VectorStorage};

fn main() -> Result<(), BatchError> {
    println!("=== EdgeVec Batch Insert Example ===\n");

    // 1. Create an HNSW index with 128 dimensions
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).expect("Failed to create index");

    println!("Created index with 128 dimensions");

    // 2. Prepare vectors for batch insertion
    // Each vector is a tuple of (id, vector_data)
    let vectors: Vec<(u64, Vec<f32>)> = (1..=1000)
        .map(|i| {
            // Generate a simple vector where each component is based on the ID
            let vector: Vec<f32> = (0..128).map(|j| ((i + j) as f32).sin()).collect();
            (i as u64, vector)
        })
        .collect();

    println!("Prepared {} vectors for insertion", vectors.len());

    // 3. Batch insert with progress tracking
    println!("\nInserting vectors with progress tracking...");

    let ids = index.batch_insert(
        vectors,
        &mut storage,
        Some(|inserted, total| {
            // Progress callback is called at ~10% intervals
            let percent = (inserted as f32 / total as f32) * 100.0;
            println!("  Progress: {}/{} ({:.0}%)", inserted, total, percent);
        }),
    )?;

    println!("\nSuccessfully inserted {} vectors", ids.len());
    println!("Index now contains {} nodes", index.node_count());

    // 4. Demonstrate batch insert without progress tracking (faster)
    println!("\n--- Second Batch (no progress tracking) ---");

    let more_vectors: Vec<(u64, Vec<f32>)> = (1001..=2000)
        .map(|i| {
            let vector: Vec<f32> = (0..128).map(|j| ((i + j) as f32).cos()).collect();
            (i as u64, vector)
        })
        .collect();

    // Pass None for progress_callback for maximum throughput
    let more_ids = index.batch_insert(
        more_vectors,
        &mut storage,
        None::<fn(usize, usize)>, // Type hint needed for None
    )?;

    println!("Inserted {} more vectors", more_ids.len());
    println!("Index now contains {} total nodes", index.node_count());

    // 5. Search to verify vectors are indexed
    println!("\n--- Search Verification ---");

    let query: Vec<f32> = (0..128).map(|j| (1.0 + j as f32).sin()).collect();
    let results = index.search(&query, 5, &storage).expect("Search failed");

    println!("Top 5 nearest neighbors:");
    for (i, result) in results.iter().enumerate() {
        println!(
            "  {}. ID: {}, Distance: {:.4}",
            i + 1,
            result.vector_id.0,
            result.distance
        );
    }

    // 6. Demonstrate error handling with best-effort semantics
    println!("\n--- Error Handling Demo ---");

    // Attempt to insert vectors with a duplicate ID
    let vectors_with_duplicate: Vec<(u64, Vec<f32>)> = vec![
        (5001, vec![0.1; 128]), // New ID - will succeed
        (1, vec![0.2; 128]),    // Duplicate ID - will be skipped
        (5002, vec![0.3; 128]), // New ID - will succeed
    ];

    let partial_ids = index.batch_insert(
        vectors_with_duplicate,
        &mut storage,
        None::<fn(usize, usize)>,
    )?;

    // Only 2 vectors inserted (duplicate skipped via best-effort semantics)
    println!(
        "Attempted 3 vectors, {} inserted (duplicate skipped)",
        partial_ids.len()
    );
    println!("Index now contains {} total nodes", index.node_count());

    println!("\n=== Example Complete ===");

    Ok(())
}
