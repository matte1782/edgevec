//! Benchmarks: Batch Insert vs Sequential Insert (W11.5)
//!
//! Run with: `cargo bench --bench batch_vs_sequential`
//!
//! # Purpose
//!
//! Validate the hypothesis that batch insertion is >=3x faster than sequential
//! insertion due to reduced function call overhead and better cache locality.
//!
//! # Reproducibility
//!
//! All benchmarks use:
//! - Seed: 42 for RNG
//! - Dimensions: 128
//! - Distribution: Uniform [-1, 1]
//!
//! # Expected Results
//!
//! **[HYPOTHESIS]**: Batch insert should be 3-5x faster than sequential.
//! - Sequential: O(n) individual insert calls
//! - Batch: O(1) function calls + O(n) internal operations
//!
//! # Benchmark Methodology
//!
//! - Uses `iter_batched` to exclude vector cloning from timing
//! - Sample size: 20 iterations for better statistical significance
//! - Throughput measured in elements/second

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use edgevec::batch::BatchInsertable;
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::storage::VectorStorage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::hint::black_box;

const SEED: u64 = 42;
const DIMS: usize = 128;
/// Sample size for statistical significance (m3 fix: increased from 10)
const SAMPLE_SIZE: usize = 20;

/// Generates deterministic test vectors.
fn generate_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect()
}

/// Generates deterministic test vectors with IDs for batch insert.
fn generate_vectors_with_ids(count: usize, dims: usize, seed: u64) -> Vec<(u64, Vec<f32>)> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (1..=count)
        .map(|i| {
            let vector: Vec<f32> = (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect();
            (i as u64, vector)
        })
        .collect()
}

/// Benchmark group: Sequential Insert
fn bench_sequential_insert(c: &mut Criterion) {
    let counts = [100, 1_000, 5_000];

    let mut group = c.benchmark_group("sequential_insert");
    group.sample_size(SAMPLE_SIZE);

    for count in counts {
        group.throughput(Throughput::Elements(count as u64));

        let vectors = generate_vectors(count, DIMS, SEED);

        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &vectors,
            |b, vectors| {
                // Sequential insert: iterate over reference (no clone needed)
                b.iter(|| {
                    let config = HnswConfig::new(DIMS as u32);
                    let mut storage = VectorStorage::new(&config, None);
                    let mut index = HnswIndex::new(config, &storage).unwrap();

                    for v in vectors {
                        index.insert(black_box(v), &mut storage).unwrap();
                    }
                    black_box(index)
                });
            },
        );
    }

    group.finish();
}

/// Benchmark group: Batch Insert
/// Uses iter_batched to exclude clone overhead from timing (m2 fix)
fn bench_batch_insert(c: &mut Criterion) {
    let counts = [100, 1_000, 5_000];

    let mut group = c.benchmark_group("batch_insert");
    group.sample_size(SAMPLE_SIZE);

    for count in counts {
        group.throughput(Throughput::Elements(count as u64));

        let vectors = generate_vectors_with_ids(count, DIMS, SEED);

        group.bench_with_input(
            BenchmarkId::from_parameter(count),
            &vectors,
            |b, vectors| {
                // m2 fix: Use iter_batched to clone in setup, not in timing
                b.iter_batched(
                    || vectors.clone(), // Setup: clone happens here (excluded from timing)
                    |batch_vectors| {
                        // Timed code: only measures actual batch_insert
                        let config = HnswConfig::new(DIMS as u32);
                        let mut storage = VectorStorage::new(&config, None);
                        let mut index = HnswIndex::new(config, &storage).unwrap();

                        let result = index.batch_insert(
                            batch_vectors,
                            &mut storage,
                            None::<fn(usize, usize)>,
                        );
                        black_box(result)
                    },
                    BatchSize::SmallInput,
                );
            },
        );
    }

    group.finish();
}

/// Benchmark group: Direct Comparison (same setup, different methods)
/// Uses iter_batched for batch insert to exclude clone overhead (m2 fix)
fn bench_comparison(c: &mut Criterion) {
    let count = 1_000;

    let mut group = c.benchmark_group("batch_vs_sequential_1k");
    group.sample_size(SAMPLE_SIZE);
    group.throughput(Throughput::Elements(count as u64));

    // Sequential baseline (no clone needed - iterates over reference)
    let vectors_seq = generate_vectors(count, DIMS, SEED);
    group.bench_function("sequential", |b| {
        b.iter(|| {
            let config = HnswConfig::new(DIMS as u32);
            let mut storage = VectorStorage::new(&config, None);
            let mut index = HnswIndex::new(config, &storage).unwrap();

            for v in &vectors_seq {
                index.insert(black_box(v), &mut storage).unwrap();
            }
            black_box(index)
        });
    });

    // Batch insert with iter_batched (m2 fix)
    let vectors_batch = generate_vectors_with_ids(count, DIMS, SEED);
    group.bench_function("batch", |b| {
        b.iter_batched(
            || vectors_batch.clone(), // Setup: clone excluded from timing
            |batch_vectors| {
                let config = HnswConfig::new(DIMS as u32);
                let mut storage = VectorStorage::new(&config, None);
                let mut index = HnswIndex::new(config, &storage).unwrap();

                let result =
                    index.batch_insert(batch_vectors, &mut storage, None::<fn(usize, usize)>);
                black_box(result)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

/// Benchmark: Memory overhead comparison
/// Uses iter_batched to exclude clone overhead from timing (m2 fix)
fn bench_memory_overhead(c: &mut Criterion) {
    let count = 1_000;

    let mut group = c.benchmark_group("memory_overhead_1k");
    group.sample_size(SAMPLE_SIZE);

    let vectors = generate_vectors_with_ids(count, DIMS, SEED);

    // Measure batch insert with progress callback (adds overhead)
    group.bench_function("batch_with_progress", |b| {
        b.iter_batched(
            || vectors.clone(),
            |batch_vectors| {
                let config = HnswConfig::new(DIMS as u32);
                let mut storage = VectorStorage::new(&config, None);
                let mut index = HnswIndex::new(config, &storage).unwrap();

                let result = index.batch_insert(
                    batch_vectors,
                    &mut storage,
                    Some(|_c, _t| {
                        // Empty callback to measure overhead
                    }),
                );
                black_box(result)
            },
            BatchSize::SmallInput,
        );
    });

    // Measure batch insert without progress callback
    group.bench_function("batch_without_progress", |b| {
        b.iter_batched(
            || vectors.clone(),
            |batch_vectors| {
                let config = HnswConfig::new(DIMS as u32);
                let mut storage = VectorStorage::new(&config, None);
                let mut index = HnswIndex::new(config, &storage).unwrap();

                let result =
                    index.batch_insert(batch_vectors, &mut storage, None::<fn(usize, usize)>);
                black_box(result)
            },
            BatchSize::SmallInput,
        );
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sequential_insert,
    bench_batch_insert,
    bench_comparison,
    bench_memory_overhead
);
criterion_main!(benches);
