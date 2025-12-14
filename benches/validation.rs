//! Benchmark Validation Suite
//!
//! Task: W10.8 - Create Benchmark Validation Suite
//!
//! This module provides automated benchmark validation to detect performance
//! regressions in CI. It runs 4 core benchmarks and compares against baselines.
//!
//! ## Benchmarks
//!
//! 1. `insert_1k` - Insert 1000 vectors (768 dimensions)
//! 2. `search_10k` - Search in 10k vector index (k=10)
//! 3. `quantization_encode` - SQ8 encoding (768 dimensions)
//! 4. `hamming_distance` - Binary Hamming distance (96 bytes)
//!
//! ## Usage
//!
//! ```bash
//! # Run validation benchmarks
//! cargo bench --bench validation
//!
//! # Run regression detection
//! python benches/check_regression.py
//! ```
//!
//! ## Baselines
//!
//! Baselines are stored in `benches/baselines.json` and contain:
//! - P50 (median) latency in nanoseconds
//! - P99 latency in nanoseconds
//! - Threshold multiplier for regression detection (default: 1.1 = 10%)

use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode};
use edgevec::hnsw::{HnswConfig, HnswIndex, SearchContext};
use edgevec::quantization::binary::QuantizedVector;
use edgevec::quantization::ScalarQuantizer;
use edgevec::storage::VectorStorage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::Duration;

// =============================================================================
// CONSTANTS
// =============================================================================

/// Seed for deterministic benchmarks (reproducibility)
const SEED: u64 = 42;

/// Dimensions for vector benchmarks (typical embedding size)
const DIMS: usize = 768;

/// Number of vectors to insert in insert_1k benchmark
const INSERT_COUNT: usize = 1000;

/// Number of vectors in the search index
const SEARCH_INDEX_SIZE: usize = 10_000;

/// Number of results to return in search
const SEARCH_K: usize = 10;

// =============================================================================
// HELPERS
// =============================================================================

/// Generates deterministic test vectors.
fn generate_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect()
}

/// Generate random QuantizedVector for benchmarking.
fn generate_quantized_vector(seed: u64) -> QuantizedVector {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut data = [0u8; 96];
    for byte in &mut data {
        *byte = rng.gen();
    }
    QuantizedVector::from_bytes(data)
}

// =============================================================================
// BENCHMARK 1: INSERT_1K
// =============================================================================

/// Benchmark: Insert 1000 vectors into HNSW index.
///
/// This measures the end-to-end insertion performance including:
/// - Vector storage allocation
/// - HNSW graph construction
/// - Neighbor selection
///
/// Target: <500ms for 1000 vectors (768 dimensions)
fn bench_insert_1k(c: &mut Criterion) {
    let vectors = generate_vectors(INSERT_COUNT, DIMS, SEED);

    let mut group = c.benchmark_group("validation");
    group.sample_size(10); // Fewer samples for expensive benchmarks
    group.measurement_time(Duration::from_secs(10));
    group.sampling_mode(SamplingMode::Flat);

    group.bench_function("insert_1k", |b| {
        b.iter(|| {
            let config = HnswConfig::new(DIMS as u32);
            let mut storage = VectorStorage::new(&config, None);
            let mut index = HnswIndex::new(config, &storage).unwrap();

            for v in &vectors {
                index.insert(black_box(v), &mut storage).unwrap();
            }
            black_box(index)
        });
    });

    group.finish();
}

// =============================================================================
// BENCHMARK 2: SEARCH_10K
// =============================================================================

/// Benchmark: Search in 10k vector index.
///
/// This measures search latency with a pre-built index:
/// - Greedy search through upper layers
/// - Beam search in base layer
/// - Result extraction
///
/// Target: <5ms for k=10 search in 10k vectors
fn bench_search_10k(c: &mut Criterion) {
    // Build index once (outside benchmark loop)
    let vectors = generate_vectors(SEARCH_INDEX_SIZE, DIMS, SEED);
    let config = HnswConfig::new(DIMS as u32);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    for v in &vectors {
        index.insert(v, &mut storage).unwrap();
    }

    // Use first vector as query
    let query = &vectors[0];

    let mut group = c.benchmark_group("validation");
    group.sample_size(100);
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("search_10k", |b| {
        let mut search_ctx = SearchContext::new();
        b.iter(|| {
            black_box(
                index
                    .search_with_context(black_box(query), SEARCH_K, &storage, &mut search_ctx)
                    .unwrap(),
            )
        });
    });

    group.finish();
}

// =============================================================================
// BENCHMARK 3: QUANTIZATION_ENCODE
// =============================================================================

/// Benchmark: SQ8 quantization encoding.
///
/// This measures the time to quantize a single vector:
/// - Min-max normalization
/// - Float to u8 conversion
///
/// Target: <10µs per 768-dimension vector
fn bench_quantization_encode(c: &mut Criterion) {
    let mut rng = ChaCha8Rng::seed_from_u64(SEED);
    let vector: Vec<f32> = (0..DIMS).map(|_| rng.gen_range(-10.0..10.0)).collect();

    // Train quantizer
    let batch = vec![vector.as_slice()];
    let quantizer = ScalarQuantizer::train(&batch);

    let mut group = c.benchmark_group("validation");
    group.sample_size(1000);
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("quantization_encode", |b| {
        b.iter(|| black_box(quantizer.quantize(black_box(&vector))))
    });

    group.finish();
}

// =============================================================================
// BENCHMARK 4: HAMMING_DISTANCE
// =============================================================================

/// Benchmark: Binary Hamming distance calculation.
///
/// This measures the core distance metric for binary vectors:
/// - XOR operation
/// - Population count
///
/// Target: <100ns per 96-byte vector pair
fn bench_hamming_distance(c: &mut Criterion) {
    let q1 = generate_quantized_vector(SEED);
    let q2 = generate_quantized_vector(SEED + 1);

    let mut group = c.benchmark_group("validation");
    group.sample_size(10000);
    group.measurement_time(Duration::from_secs(5));

    group.bench_function("hamming_distance", |b| {
        b.iter(|| black_box(q1.hamming_distance(black_box(&q2))))
    });

    group.finish();
}

// =============================================================================
// CRITERION CONFIGURATION
// =============================================================================

criterion_group! {
    name = validation_benches;
    config = Criterion::default()
        .with_output_color(true)
        .without_plots(); // Disable HTML plots for CI speed
    targets =
        bench_insert_1k,
        bench_search_10k,
        bench_quantization_encode,
        bench_hamming_distance
}

criterion_main!(validation_benches);
