//! Test that replicates the exact WASM demo flow to find F32 crash.
//!
//! The WASM demo:
//! 1. Creates EdgeVec with config
//! 2. Calls enableBQ() before any inserts
//! 3. Inserts vectors with metadata
//! 4. Does F32 search → CRASHES

use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::metadata::MetadataValue;
use edgevec::storage::VectorStorage;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

/// Test that replicates WASM demo flow exactly.
#[test]
fn test_wasm_demo_flow_f32_search() {
    const DIM: u32 = 768; // Same as demo
    const NUM_VECTORS: usize = 800;
    const K: usize = 10;

    // Step 1: Create config (like EdgeVecConfig in WASM)
    let mut config = HnswConfig::new(DIM);
    config.m = 16;
    config.m0 = 32;
    config.ef_construction = 200;
    config.ef_search = 50;

    // Step 2: Create storage and index (like EdgeVec::new)
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).expect("Index creation failed");

    // Step 3: Enable BQ BEFORE any inserts (like demo does)
    index.enable_bq(&storage).expect("Enable BQ failed");
    assert!(index.has_bq(), "BQ should be enabled");

    let categories = ["tech", "science", "art", "music", "sports"];
    let mut rng = StdRng::seed_from_u64(42);

    // Step 4: Insert vectors with metadata (like demo's generateData)
    for i in 0..NUM_VECTORS {
        // Generate random normalized vector
        let mut v: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-0.5..0.5)).collect();
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        for x in &mut v {
            *x /= norm;
        }

        // Insert with metadata
        let metadata = {
            let mut m = std::collections::HashMap::new();
            m.insert(
                "category".to_string(),
                MetadataValue::String(categories[i % 5].to_string()),
            );
            m.insert(
                "score".to_string(),
                MetadataValue::Float(rng.gen_range(0.0..1.0)),
            );
            m.insert("active".to_string(), MetadataValue::Boolean(rng.gen()));
            m
        };

        index
            .insert_with_metadata(&mut storage, &v, metadata)
            .expect("Insert failed");
    }

    println!("Inserted {} vectors", index.len());
    println!("Storage len: {}", storage.len());
    println!("Has BQ: {}", index.has_bq());

    // Step 5: Generate random query (like demo does)
    let mut query: Vec<f32> = (0..DIM).map(|_| rng.gen_range(-0.5..0.5)).collect();
    let norm: f32 = query.iter().map(|x| x * x).sum::<f32>().sqrt();
    for x in &mut query {
        *x /= norm;
    }

    // Step 6: F32 search - THIS IS WHAT CRASHES IN WASM
    println!("Starting F32 search...");
    let results = index
        .search(&query, K, &storage)
        .expect("F32 search failed");

    println!("F32 search returned {} results", results.len());
    assert!(!results.is_empty(), "Should have results");

    for (i, r) in results.iter().enumerate() {
        println!("  {}: id={}, distance={:.4}", i, r.vector_id.0, r.distance);
    }

    // Also test BQ search to confirm it works
    println!("Starting BQ search...");
    let bq_results = index
        .search_bq(&query, K, &storage)
        .expect("BQ search failed");
    println!("BQ search returned {} results", bq_results.len());

    println!("All searches passed!");
}
