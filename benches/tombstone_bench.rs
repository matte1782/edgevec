//! Tombstone Search Performance Benchmark (AC16.3.4 Verification)
//!
//! This benchmark validates the performance requirement from W16.3:
//! **Search performance degradation <20% at 10% tombstone ratio**
//!
//! # Methodology (FIXED per HOSTILE_REVIEWER M1)
//!
//! **Key Fix:** Use SINGLE index with incremental deletes to eliminate rebuild variance.
//!
//! 1. Build ONE index with N vectors
//! 2. Warmup: Run 50 queries (discarded)
//! 3. Baseline: Run 100 queries, record P99 latency
//! 4. Delete 10% of vectors (incremental, same index)
//! 5. Run same 100 queries, record P99 latency
//! 6. Calculate: `degradation = (tombstone_p99 - baseline_p99) / baseline_p99 * 100`
//! 7. PASS if degradation < 20% at 10% tombstones
//!
//! # Why This Matters
//!
//! Previous methodology rebuilt indexes for each tombstone ratio, introducing:
//! - Different RNG states for graph construction
//! - Different memory layouts
//! - Cache effects from rebuilds
//!
//! This version uses the SAME index throughout, measuring the TRUE impact of tombstones.
//!
//! # Run Commands
//!
//! Quick validation (10k vectors):
//! ```bash
//! cargo bench --bench tombstone_bench
//! ```
//!
//! Full validation (100k vectors, slower):
//! ```bash
//! TOMBSTONE_BENCH_FULL=1 cargo bench --bench tombstone_bench
//! ```

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode};
use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::storage::VectorStorage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::hint::black_box;
use std::time::{Duration, Instant};

/// Fixed seed for reproducibility (per spec)
const SEED: u64 = 42;

/// Dimensions for benchmark vectors (per spec: 128)
const DIMS: u32 = 128;

/// Number of nearest neighbors to search (per spec: k=10)
const K: usize = 10;

/// Number of search queries for P99 calculation
const QUERY_COUNT: usize = 100;

/// Number of warmup queries to stabilize cache/JIT
const WARMUP_COUNT: usize = 50;

/// Number of P99 measurement rounds for stability
const P99_ROUNDS: usize = 5;

/// Get vector count from environment or default
fn get_vector_count() -> usize {
    if std::env::var("TOMBSTONE_BENCH_FULL").is_ok() {
        100_000 // Full spec: 100k vectors
    } else {
        10_000 // Quick CI: 10k vectors
    }
}

/// Generates deterministic test vectors with fixed seed
fn generate_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect()
}

/// Generates deterministic query vectors (separate seed to avoid overlap)
fn generate_queries(count: usize, dims: usize) -> Vec<Vec<f32>> {
    generate_vectors(count, dims, SEED + 1000)
}

/// Build index from vectors
fn build_index(vectors: &[Vec<f32>], dims: u32) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(dims);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    for v in vectors {
        index.insert(v, &mut storage).unwrap();
    }
    (index, storage)
}

/// Run warmup queries to stabilize cache and JIT
fn warmup(index: &HnswIndex, storage: &VectorStorage, queries: &[Vec<f32>], k: usize) {
    for q in queries.iter().take(WARMUP_COUNT) {
        let _ = black_box(index.search(q, k, storage).unwrap());
    }
}

/// Measure search latencies and return P99
/// Uses all queries and returns the P99 latency
fn measure_p99_latency(
    index: &HnswIndex,
    storage: &VectorStorage,
    queries: &[Vec<f32>],
    k: usize,
) -> Duration {
    let mut latencies: Vec<Duration> = queries
        .iter()
        .map(|q| {
            let start = Instant::now();
            let _ = black_box(index.search(q, k, storage).unwrap());
            start.elapsed()
        })
        .collect();

    // Sort and get P99
    latencies.sort();
    let p99_idx = (latencies.len() as f64 * 0.99).ceil() as usize - 1;
    latencies[p99_idx.min(latencies.len() - 1)]
}

/// Measure P99 multiple times and return (mean, min, max) for stability
fn measure_p99_stable(
    index: &HnswIndex,
    storage: &VectorStorage,
    queries: &[Vec<f32>],
    k: usize,
    rounds: usize,
) -> (Duration, Duration, Duration) {
    let measurements: Vec<Duration> = (0..rounds)
        .map(|_| measure_p99_latency(index, storage, queries, k))
        .collect();

    let min = *measurements.iter().min().unwrap();
    let max = *measurements.iter().max().unwrap();
    let sum: Duration = measurements.iter().sum();
    let mean = sum / rounds as u32;

    (mean, min, max)
}

/// Calculate percentage degradation
fn calc_degradation(baseline: Duration, with_tombstones: Duration) -> f64 {
    let baseline_ns = baseline.as_nanos() as f64;
    let tombstone_ns = with_tombstones.as_nanos() as f64;
    ((tombstone_ns - baseline_ns) / baseline_ns) * 100.0
}

/// FIXED: Main benchmark using SINGLE index with incremental deletes
///
/// This addresses M1 from HOSTILE_REVIEWER: benchmark methodology was flawed
/// because it rebuilt indexes for each tombstone ratio.
fn bench_tombstone_incremental(c: &mut Criterion) {
    let count = get_vector_count();
    let dims = DIMS;

    println!("\n=== Tombstone Performance Benchmark (FIXED METHODOLOGY) ===");
    println!("Vector count: {}", count);
    println!("Dimensions: {}", dims);
    println!("k: {}", K);
    println!("Query count: {}", QUERY_COUNT);
    println!("Warmup queries: {}", WARMUP_COUNT);
    println!("P99 rounds: {}", P99_ROUNDS);
    println!("Seed: {}", SEED);
    println!();
    println!("KEY: Using SINGLE index with incremental deletes (no rebuild variance)");
    println!();

    // Generate data
    let vectors = generate_vectors(count, dims as usize, SEED);
    let queries = generate_queries(QUERY_COUNT, dims as usize);

    // Build SINGLE index (used throughout)
    let (mut index, storage) = build_index(&vectors, dims);

    // Warmup to stabilize cache/JIT
    warmup(&index, &storage, &queries, K);

    // Measure baseline (0% tombstones)
    let (baseline_mean, baseline_min, baseline_max) =
        measure_p99_stable(&index, &storage, &queries, K, P99_ROUNDS);

    println!(
        "Baseline P99 (0% tombstones): {:?} (min: {:?}, max: {:?})",
        baseline_mean, baseline_min, baseline_max
    );

    // Tombstone ratios to test (cumulative deletes on same index)
    let ratios = [10, 25, 50];
    let mut total_deleted = 0usize;

    let mut group = c.benchmark_group("tombstone_incremental");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(50);

    // Benchmark baseline first
    group.bench_function("search_0pct", |b| {
        // Note: This runs AFTER deletions below in criterion's execution order,
        // so we need a separate approach. We'll benchmark the current state.
        b.iter(|| {
            let query = &queries[0];
            black_box(index.search(black_box(query), K, &storage).unwrap())
        });
    });

    group.finish();

    // Now do incremental deletes and measure
    for &ratio in &ratios {
        let target_deleted = count * ratio / 100;
        let to_delete = target_deleted - total_deleted;

        // Delete incrementally (same index!)
        for i in (total_deleted + 1)..=(total_deleted + to_delete) {
            index
                .soft_delete(VectorId(i as u64))
                .expect("Delete should succeed");
        }
        total_deleted = target_deleted;

        // Verify
        assert_eq!(
            index.deleted_count(),
            target_deleted,
            "Expected {} deletions, got {}",
            target_deleted,
            index.deleted_count()
        );

        // Warmup after state change
        warmup(&index, &storage, &queries, K);

        // Measure P99 (stable with multiple rounds)
        let (p99_mean, p99_min, p99_max) =
            measure_p99_stable(&index, &storage, &queries, K, P99_ROUNDS);

        let degradation = calc_degradation(baseline_mean, p99_mean);

        println!(
            "{}% tombstones P99: {:?} (min: {:?}, max: {:?}) - degradation: {:.1}%",
            ratio, p99_mean, p99_min, p99_max, degradation
        );

        // AC16.3.4 validation at 10% tombstones
        if ratio == 10 {
            if degradation < 20.0 {
                println!("  ✅ AC16.3.4 PASS: {:.1}% < 20% threshold", degradation);
            } else {
                println!("  ❌ AC16.3.4 FAIL: {:.1}% >= 20% threshold", degradation);
            }
        }
    }

    println!();
}

/// Criterion benchmark with stable index states
/// Creates indexes upfront to avoid rebuild during benchmark
fn bench_tombstone_criterion(c: &mut Criterion) {
    let count = get_vector_count();
    let dims = DIMS;

    let vectors = generate_vectors(count, dims as usize, SEED);
    let queries = generate_queries(QUERY_COUNT, dims as usize);

    // Pre-build indexes at each tombstone level for fair criterion comparison
    // This is separate from the incremental test above

    let mut group = c.benchmark_group("tombstone_search_criterion");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(50);
    group.warm_up_time(Duration::from_secs(2));

    for ratio in [0, 10, 25, 50] {
        // Build fresh index for this ratio
        let (mut index, storage) = build_index(&vectors, dims);

        // Apply deletions
        if ratio > 0 {
            let delete_count = count * ratio / 100;
            for i in 1..=delete_count {
                index.soft_delete(VectorId(i as u64)).unwrap();
            }
        }

        group.bench_with_input(
            BenchmarkId::new("search", format!("{}pct_tombstones", ratio)),
            &ratio,
            |b, _| {
                b.iter(|| {
                    let query = &queries[0];
                    black_box(index.search(black_box(query), K, &storage).unwrap())
                });
            },
        );
    }

    group.finish();
}

/// AC16.3.4 validation with proper methodology
/// Uses SINGLE index with incremental deletion
fn validate_ac16_3_4_fixed(c: &mut Criterion) {
    let count = get_vector_count();
    let dims = DIMS;

    let vectors = generate_vectors(count, dims as usize, SEED);
    let queries = generate_queries(QUERY_COUNT, dims as usize);

    // Build SINGLE index
    let (mut index, storage) = build_index(&vectors, dims);

    // Warmup
    warmup(&index, &storage, &queries, K);

    // Measure baseline
    let (baseline_p99, baseline_min, baseline_max) =
        measure_p99_stable(&index, &storage, &queries, K, P99_ROUNDS);

    // Delete 10% (incremental on same index)
    let delete_count = count / 10;
    for i in 1..=delete_count {
        index
            .soft_delete(VectorId(i as u64))
            .expect("Delete should succeed");
    }

    // Warmup after deletion
    warmup(&index, &storage, &queries, K);

    // Measure with tombstones
    let (tombstone_p99, tombstone_min, tombstone_max) =
        measure_p99_stable(&index, &storage, &queries, K, P99_ROUNDS);

    let degradation = calc_degradation(baseline_p99, tombstone_p99);

    println!("\n=== AC16.3.4 Validation (FIXED METHODOLOGY) ===");
    println!(
        "Baseline P99: {:?} (range: {:?} - {:?})",
        baseline_p99, baseline_min, baseline_max
    );
    println!(
        "10% Tombstone P99: {:?} (range: {:?} - {:?})",
        tombstone_p99, tombstone_min, tombstone_max
    );
    println!("Degradation: {:.2}%", degradation);
    println!(
        "Result: {} (threshold: <20%)",
        if degradation < 20.0 {
            "PASS ✅"
        } else {
            "FAIL ❌"
        }
    );
    println!();

    // Criterion group for CI tracking
    let mut group = c.benchmark_group("ac16_3_4_validation_fixed");
    group.sampling_mode(SamplingMode::Flat);
    group.sample_size(30);
    group.warm_up_time(Duration::from_secs(2));

    // For criterion, we need stable indexes, so rebuild for benchmark portion
    let (index_baseline, storage_baseline) = build_index(&vectors, dims);
    let (mut index_10pct, storage_10pct) = build_index(&vectors, dims);
    for i in 1..=delete_count {
        index_10pct.soft_delete(VectorId(i as u64)).unwrap();
    }

    group.bench_function("baseline_0pct", |b| {
        b.iter(|| {
            let query = &queries[0];
            black_box(
                index_baseline
                    .search(black_box(query), K, &storage_baseline)
                    .unwrap(),
            )
        });
    });

    group.bench_function("tombstone_10pct", |b| {
        b.iter(|| {
            let query = &queries[0];
            black_box(
                index_10pct
                    .search(black_box(query), K, &storage_10pct)
                    .unwrap(),
            )
        });
    });

    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(5))
        .warm_up_time(Duration::from_secs(2));
    targets = bench_tombstone_incremental, bench_tombstone_criterion, validate_ac16_3_4_fixed
);
criterion_main!(benches);
