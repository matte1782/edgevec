//! P99 Latency Tracking Benchmark (W19.4)
//!
//! This benchmark measures and reports P50, P99, and P999 percentiles for
//! search latency. It provides detailed latency distribution analysis for
//! performance monitoring and regression detection.
//!
//! ## Usage
//!
//! ```bash
//! # Run P99 benchmark
//! cargo bench --bench p99_bench
//!
//! # Run with detailed output
//! cargo bench --bench p99_bench -- --verbose
//! ```
//!
//! ## Output
//!
//! The benchmark outputs latency percentiles:
//! - P50 (median): 50th percentile
//! - P99: 99th percentile
//! - P999: 99.9th percentile
//! - Max: Maximum observed latency
//!
//! ## CI Integration
//!
//! This benchmark is run as part of the regression.yml workflow to track
//! tail latency over time and detect performance degradation.

use criterion::{black_box, criterion_group, criterion_main, Criterion, SamplingMode};
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::storage::VectorStorage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::time::{Duration, Instant};

// =============================================================================
// CONSTANTS
// =============================================================================

/// Seed for deterministic benchmarks (reproducibility)
const SEED: u64 = 42;

/// Dimensions for vector benchmarks (typical embedding size)
const DIMS: u32 = 128;

/// Number of vectors in the search index
const INDEX_SIZE: usize = 10_000;

/// Number of search queries to run for percentile calculation
const QUERY_COUNT: usize = 1000;

/// Number of results to return in search
const SEARCH_K: usize = 10;

// =============================================================================
// HELPERS
// =============================================================================

/// Generates deterministic test vectors.
fn generate_vectors(count: usize, dims: u32, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect()
}

/// Calculate percentile from sorted latencies.
fn percentile(sorted_latencies: &[f64], p: f64) -> f64 {
    if sorted_latencies.is_empty() {
        return 0.0;
    }
    let idx = ((p / 100.0) * sorted_latencies.len() as f64) as usize;
    let idx = idx.min(sorted_latencies.len() - 1);
    sorted_latencies[idx]
}

/// Collect search latencies for percentile analysis.
fn collect_latencies(index: &HnswIndex, storage: &VectorStorage, queries: &[Vec<f32>]) -> Vec<f64> {
    let mut latencies = Vec::with_capacity(queries.len());

    for query in queries {
        let start = Instant::now();
        let _ = black_box(index.search(query, SEARCH_K, storage));
        latencies.push(start.elapsed().as_nanos() as f64);
    }

    latencies.sort_by(|a, b| a.partial_cmp(b).unwrap());
    latencies
}

// =============================================================================
// P99 BENCHMARK
// =============================================================================

/// Benchmark: P50/P99/P999 search latency on 10k index.
///
/// This benchmark:
/// 1. Builds a 10k vector index (128 dimensions for faster builds)
/// 2. Runs 1000 search queries
/// 3. Reports percentile latencies
///
/// The benchmark uses Criterion's iter_custom for precise latency collection.
fn bench_p99_latency(c: &mut Criterion) {
    // Build index with 10k vectors
    let config = HnswConfig::new(DIMS);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let vectors = generate_vectors(INDEX_SIZE, DIMS, SEED);
    for vector in &vectors {
        index.insert(vector, &mut storage).unwrap();
    }

    // Generate query vectors
    let queries = generate_vectors(QUERY_COUNT, DIMS, SEED + 1);

    let mut group = c.benchmark_group("p99_latency");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));
    group.sampling_mode(SamplingMode::Flat);

    group.bench_function("search_10k_percentiles", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;

            for _ in 0..iters {
                let latencies = collect_latencies(&index, &storage, &queries);

                // Calculate percentiles
                let p50 = percentile(&latencies, 50.0);
                let p99 = percentile(&latencies, 99.0);
                let p999 = percentile(&latencies, 99.9);
                let max = latencies.last().copied().unwrap_or(0.0);

                // Print percentiles on each iteration for visibility
                println!(
                    "  Latencies: P50={:.2}µs, P99={:.2}µs, P999={:.2}µs, Max={:.2}µs",
                    p50 / 1000.0,
                    p99 / 1000.0,
                    p999 / 1000.0,
                    max / 1000.0
                );

                // Use P99 as the benchmark metric
                total_duration += Duration::from_nanos(p99 as u64);
            }

            total_duration
        });
    });

    group.finish();
}

/// Benchmark: P99 latency with tombstones (30% deleted).
///
/// This tests tail latency under realistic conditions where some
/// vectors have been deleted but not yet compacted.
fn bench_p99_with_tombstones(c: &mut Criterion) {
    // Build index with 10k vectors
    let config = HnswConfig::new(DIMS);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    let vectors = generate_vectors(INDEX_SIZE, DIMS, SEED);
    let mut ids = Vec::new();
    for vector in &vectors {
        let id = index.insert(vector, &mut storage).unwrap();
        ids.push(id);
    }

    // Delete 30% of vectors
    for id in ids.iter().take(INDEX_SIZE * 30 / 100) {
        index.soft_delete(*id).unwrap();
    }

    // Generate query vectors
    let queries = generate_vectors(QUERY_COUNT, DIMS, SEED + 2);

    let mut group = c.benchmark_group("p99_latency");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(30));
    group.sampling_mode(SamplingMode::Flat);

    group.bench_function("search_10k_30pct_tombstones", |b| {
        b.iter_custom(|iters| {
            let mut total_duration = Duration::ZERO;

            for _ in 0..iters {
                let latencies = collect_latencies(&index, &storage, &queries);

                let p50 = percentile(&latencies, 50.0);
                let p99 = percentile(&latencies, 99.0);
                let p999 = percentile(&latencies, 99.9);
                let max = latencies.last().copied().unwrap_or(0.0);

                println!(
                    "  [30% tombstones] P50={:.2}µs, P99={:.2}µs, P999={:.2}µs, Max={:.2}µs",
                    p50 / 1000.0,
                    p99 / 1000.0,
                    p999 / 1000.0,
                    max / 1000.0
                );

                total_duration += Duration::from_nanos(p99 as u64);
            }

            total_duration
        });
    });

    group.finish();
}

/// Benchmark: P99 latency at different index sizes.
///
/// Tracks how tail latency scales with index size.
fn bench_p99_scaling(c: &mut Criterion) {
    let sizes = [1_000, 5_000, 10_000, 25_000];
    let queries = generate_vectors(100, DIMS, SEED + 3); // Fewer queries for speed

    let mut group = c.benchmark_group("p99_scaling");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(20));
    group.sampling_mode(SamplingMode::Flat);

    for size in sizes {
        // Build index
        let config = HnswConfig::new(DIMS);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).unwrap();

        let vectors = generate_vectors(size, DIMS, SEED);
        for vector in &vectors {
            index.insert(vector, &mut storage).unwrap();
        }

        group.bench_function(format!("search_{}_p99", size), |b| {
            b.iter_custom(|iters| {
                let mut total_duration = Duration::ZERO;

                for _ in 0..iters {
                    let latencies = collect_latencies(&index, &storage, &queries);
                    let p99 = percentile(&latencies, 99.0);
                    total_duration += Duration::from_nanos(p99 as u64);
                }

                total_duration
            });
        });
    }

    group.finish();
}

// =============================================================================
// CRITERION CONFIGURATION
// =============================================================================

criterion_group!(
    p99_benches,
    bench_p99_latency,
    bench_p99_with_tombstones,
    bench_p99_scaling
);

criterion_main!(p99_benches);
