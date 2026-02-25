use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::metric::L2Squared;
use edgevec::metric::Metric;
use edgevec::quantization::QuantizerConfig;
use edgevec::storage::{StorageType, VectorStorage};
use rand::rngs::StdRng;
use rand::{Rng, RngExt, SeedableRng};
use std::time::Instant;

/// Verification Test for Quantized Index Recall (W6D30)
///
/// Goal: Prove that 8-bit quantization (SQ8) maintains high recall compared to Float32 baseline.
///
/// Usage:
/// This test is computationally intensive. Run in RELEASE mode.
/// `cargo test --release --test integration_quantized_recall -- --nocapture`
///
/// Environment Variables:
/// `NUM_VECTORS`: Override number of vectors (default: 10000).
#[test]
fn test_quantized_recall_comparison() {
    // 1. Constants
    let num_vectors = std::env::var("NUM_VECTORS")
        .unwrap_or_else(|_| "10000".to_string())
        .parse::<usize>()
        .unwrap();

    // Check profile
    #[cfg(debug_assertions)]
    {
        if num_vectors > 1000 {
            println!("WARNING: Running large scale test in DEBUG mode. Expect slow execution.");
            println!("Run with `--release` for accurate performance benchmarks.");
        }
    }

    const DIM: usize = 768;
    const NUM_QUERIES: usize = 100;
    const SEED: u64 = 42;

    // Data Range [0, 1] matches Quantizer [0, 1] for optimal SQ8 usage
    const DATA_MIN: f32 = 0.0;
    const DATA_MAX: f32 = 1.0;

    println!("Starting Quantized Recall Verification...");
    println!("Dimensions: {}", DIM);
    println!("Vectors: {}", num_vectors);

    // 2. Generate Data (Deterministic)
    let mut rng = StdRng::seed_from_u64(SEED);

    let mut vectors = Vec::with_capacity(num_vectors);
    for _ in 0..num_vectors {
        let vec: Vec<f32> = (0..DIM)
            .map(|_| rng.random_range(DATA_MIN..DATA_MAX))
            .collect();
        vectors.push(vec);
    }

    let mut queries = Vec::with_capacity(NUM_QUERIES);
    for _ in 0..NUM_QUERIES {
        let query: Vec<f32> = (0..DIM)
            .map(|_| rng.random_range(DATA_MIN..DATA_MAX))
            .collect();
        queries.push(query);
    }

    // 3. Verify Metric Correlation (Sanity Check)
    {
        println!("Verifying Quantization Order Preservation...");
        let q_config = QuantizerConfig {
            min: DATA_MIN,
            max: DATA_MAX,
        };
        let quantizer = edgevec::quantization::ScalarQuantizer::new(q_config);

        let idx_a = 0;
        let idx_b = 1;
        let vec_a = &vectors[idx_a];
        let vec_b = &vectors[idx_b];

        let dist_f32 = L2Squared::distance(vec_a, vec_b);
        let q_a = quantizer.quantize(vec_a);
        let q_b = quantizer.quantize(vec_b);

        // Approximate L2SquaredU8 calculation for verification
        let dist_u8_approx: u32 = q_a
            .iter()
            .zip(q_b.iter())
            .map(|(a, b)| {
                let d = (*a as i32) - (*b as i32);
                (d * d) as u32
            })
            .sum();

        println!("Dist Sample: f32={:.4}, u8={}", dist_f32, dist_u8_approx);

        // Expected Ratio: 255^2 = 65025
        let ratio = dist_u8_approx as f32 / dist_f32;
        println!("Ratio: {:.2} (Expected ~65025)", ratio);

        if (ratio - 65025.0).abs() > 10000.0 {
            println!("WARNING: Quantization metric scaling seems off.");
        }
    }

    // 4. Ground Truth (Brute Force)
    println!("Computing Ground Truth...");
    let start_gt = Instant::now();
    let mut ground_truth = Vec::with_capacity(NUM_QUERIES);

    for query in &queries {
        let mut best_dist = f32::MAX;
        let mut best_idx = 0;

        for (i, vec) in vectors.iter().enumerate() {
            let dist = L2Squared::distance(query, vec);
            if dist < best_dist {
                best_dist = dist;
                best_idx = i;
            }
        }
        ground_truth.push((best_idx + 1) as u64); // 1-based ID
    }
    println!("Ground Truth computed in {:?}", start_gt.elapsed());

    // 5. Baseline (Float32) Index
    println!("Building Baseline Float32 Index...");
    let mut config = HnswConfig::new(DIM as u32);
    config.m = 24;
    config.m0 = 48;
    config.ef_construction = 128;
    config.ef_search = 100;

    let mut baseline_storage = VectorStorage::new(&config, None);
    let mut baseline_index =
        HnswIndex::new(config.clone(), &baseline_storage).expect("Baseline index creation failed");

    let start_base_insert = Instant::now();
    for vec in &vectors {
        baseline_index
            .insert(vec, &mut baseline_storage)
            .expect("Baseline insert failed");
    }
    println!("Baseline Insert time: {:?}", start_base_insert.elapsed());

    // Measure Baseline Recall
    let mut baseline_matches = 0;
    for (i, query) in queries.iter().enumerate() {
        let results = baseline_index
            .search(query, 1, &baseline_storage)
            .expect("Baseline search failed");
        if let Some(res) = results.first() {
            if res.vector_id.0 == ground_truth[i] {
                baseline_matches += 1;
            }
        }
    }
    let baseline_recall = baseline_matches as f32 / NUM_QUERIES as f32;
    println!(
        "Baseline Recall: {:.2}% ({}/{})",
        baseline_recall * 100.0,
        baseline_matches,
        NUM_QUERIES
    );

    // 6. Quantized (U8) Index
    println!("Building Quantized U8 Index...");
    let mut quantized_storage = VectorStorage::new(&config, None);

    // Configure Quantization
    quantized_storage.set_storage_type(StorageType::QuantizedU8(QuantizerConfig {
        min: DATA_MIN,
        max: DATA_MAX,
    }));

    let mut quantized_index = HnswIndex::new(config.clone(), &quantized_storage)
        .expect("Quantized index creation failed");

    let start_quant_insert = Instant::now();
    for vec in &vectors {
        quantized_index
            .insert(vec, &mut quantized_storage)
            .expect("Quantized insert failed");
    }
    println!("Quantized Insert time: {:?}", start_quant_insert.elapsed());

    // Measure Quantized Recall
    let mut quantized_matches = 0;

    for (i, query) in queries.iter().enumerate() {
        let results = quantized_index
            .search(query, 1, &quantized_storage)
            .expect("Quantized search failed");
        if let Some(res) = results.first() {
            if res.vector_id.0 == ground_truth[i] {
                quantized_matches += 1;
            }
        }
    }
    let quantized_recall = quantized_matches as f32 / NUM_QUERIES as f32;
    println!(
        "Quantized Recall: {:.2}% ({}/{})",
        quantized_recall * 100.0,
        quantized_matches,
        NUM_QUERIES
    );

    // 7. Report and Assert
    let drop = baseline_recall - quantized_recall;
    println!("Recall Drop: {:.2}%", drop * 100.0);

    let threshold = baseline_recall * 0.90;
    println!("Threshold (90% of Baseline): {:.2}%", threshold * 100.0);

    // Warn if recall is generally poor (independent of drop)
    if baseline_recall < 0.50 {
        println!(
            "WARNING: Baseline recall is low ({:.2}%). Consider tuning HNSW parameters.",
            baseline_recall * 100.0
        );
    }

    assert!(
        quantized_recall >= threshold,
        "Quantized Recall ({:.2}%) dropped too much compared to Baseline ({:.2}%). Limit: > {:.2}%",
        quantized_recall * 100.0,
        baseline_recall * 100.0,
        threshold * 100.0
    );
}
