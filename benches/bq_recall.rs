//! Recall Benchmark for BQ Search.
//!
//! Run with: `cargo bench --bench bq_recall`
//!
//! ## Metrics Measured
//!
//! | Metric | Target (RFC-002) |
//! |:-------|:-----------------|
//! | Raw BQ recall@10 | 0.70-0.85 (acceptable) |
//! | BQ+rescore recall@10 | >0.90 (required) |
//! | BQ+rescore recall@10 (factor=5) | >0.95 (target) |
//!
//! ## Methodology
//!
//! 1. Create index with random vectors in [-1, 1] range
//! 2. Generate random queries
//! 3. For each query:
//!    - Compute F32 search as ground truth
//!    - Compute BQ/BQ+rescore search
//!    - Measure recall = |intersection| / k
//! 4. Report average recall over all queries

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::storage::VectorStorage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::collections::HashSet;

/// Generate a random f32 vector with values in [-1, 1].
fn generate_vector(dims: usize, seed: u64) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

/// Create a BQ-enabled index with n random vectors.
fn create_bq_index(
    n: usize,
    dims: u32,
    ef_construction: u32,
    ef_search: u32,
    seed: u64,
) -> (HnswIndex, VectorStorage) {
    let mut config = HnswConfig::new(dims);
    config.m = 16;
    config.m0 = 32;
    config.ef_construction = ef_construction;
    config.ef_search = ef_search;

    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for _ in 0..n {
        let v: Vec<f32> = (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect();
        index.insert_bq(&v, &mut storage).expect("Insert failed");
    }

    (index, storage)
}

/// Compute recall between two result sets.
fn compute_recall(ground_truth: &HashSet<u64>, results: &HashSet<u64>, k: usize) -> f64 {
    let intersection = ground_truth.intersection(results).count();
    intersection as f64 / k as f64
}

// ============================================================================
// RECALL BENCHMARKS
// ============================================================================

/// Benchmark recall for different rescore factors.
fn bench_recall_by_rescore_factor(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall_by_rescore_factor");
    group.sample_size(10); // Reduce samples since recall measurement is slow

    let dims = 128u32;
    let n = 1_000;
    let k = 10;
    let num_queries = 50;

    let (index, storage) = create_bq_index(n, dims, 100, 64, 42);

    // Pre-generate queries
    let queries: Vec<Vec<f32>> = (0..num_queries)
        .map(|i| generate_vector(dims as usize, 5000 + i))
        .collect();

    for factor in [1, 2, 3, 5, 10, 20] {
        group.bench_with_input(
            BenchmarkId::new("rescore_factor", factor),
            &factor,
            |bench, &factor| {
                bench.iter(|| {
                    let mut total_recall = 0.0;

                    for query in &queries {
                        // Ground truth: F32 search
                        let f32_results = index.search(query, k, &storage).unwrap();
                        let f32_ids: HashSet<u64> =
                            f32_results.iter().map(|r| r.vector_id.0).collect();

                        // BQ+rescore search
                        let bq_results = index
                            .search_bq_rescored(query, k, factor, &storage)
                            .unwrap();
                        let bq_ids: HashSet<u64> = bq_results.iter().map(|(id, _)| id.0).collect();

                        total_recall += compute_recall(&f32_ids, &bq_ids, k);
                    }

                    total_recall / num_queries as f64
                })
            },
        );
    }

    group.finish();
}

/// Benchmark raw BQ recall vs BQ+rescore.
fn bench_recall_bq_vs_rescored(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall_bq_vs_rescored");
    group.sample_size(10);

    let dims = 128u32;
    let n = 1_000;
    let k = 10;
    let num_queries = 50;

    let (index, storage) = create_bq_index(n, dims, 100, 64, 42);

    let queries: Vec<Vec<f32>> = (0..num_queries)
        .map(|i| generate_vector(dims as usize, 5000 + i))
        .collect();

    group.bench_function("raw_bq", |bench| {
        bench.iter(|| {
            let mut total_recall = 0.0;

            for query in &queries {
                let f32_results = index.search(query, k, &storage).unwrap();
                let f32_ids: HashSet<u64> = f32_results.iter().map(|r| r.vector_id.0).collect();

                let bq_results = index.search_bq(query, k, &storage).unwrap();
                let bq_ids: HashSet<u64> = bq_results.iter().map(|(id, _)| id.0).collect();

                total_recall += compute_recall(&f32_ids, &bq_ids, k);
            }

            total_recall / num_queries as f64
        })
    });

    group.bench_function("bq_rescored_3x", |bench| {
        bench.iter(|| {
            let mut total_recall = 0.0;

            for query in &queries {
                let f32_results = index.search(query, k, &storage).unwrap();
                let f32_ids: HashSet<u64> = f32_results.iter().map(|r| r.vector_id.0).collect();

                let bq_results = index.search_bq_rescored(query, k, 3, &storage).unwrap();
                let bq_ids: HashSet<u64> = bq_results.iter().map(|(id, _)| id.0).collect();

                total_recall += compute_recall(&f32_ids, &bq_ids, k);
            }

            total_recall / num_queries as f64
        })
    });

    group.finish();
}

/// Benchmark recall by index size.
fn bench_recall_by_index_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall_by_index_size");
    group.sample_size(10);

    let dims = 128u32;
    let k = 10;
    let num_queries = 30;

    for n in [500, 1_000, 2_000] {
        let (index, storage) = create_bq_index(n, dims, 100, 64, 42);

        let queries: Vec<Vec<f32>> = (0..num_queries)
            .map(|i| generate_vector(dims as usize, 5000 + i))
            .collect();

        group.bench_with_input(BenchmarkId::new("bq_rescored", n), &n, |bench, _| {
            bench.iter(|| {
                let mut total_recall = 0.0;

                for query in &queries {
                    let f32_results = index.search(query, k, &storage).unwrap();
                    let f32_ids: HashSet<u64> = f32_results.iter().map(|r| r.vector_id.0).collect();

                    let bq_results = index.search_bq_rescored(query, k, 5, &storage).unwrap();
                    let bq_ids: HashSet<u64> = bq_results.iter().map(|(id, _)| id.0).collect();

                    total_recall += compute_recall(&f32_ids, &bq_ids, k);
                }

                total_recall / num_queries as f64
            })
        });
    }

    group.finish();
}

/// Benchmark recall at varying k.
fn bench_recall_at_k(c: &mut Criterion) {
    let mut group = c.benchmark_group("recall_at_k");
    group.sample_size(10);

    let dims = 128u32;
    let n = 1_000;
    let num_queries = 30;

    let (index, storage) = create_bq_index(n, dims, 100, 64, 42);

    let queries: Vec<Vec<f32>> = (0..num_queries)
        .map(|i| generate_vector(dims as usize, 5000 + i))
        .collect();

    for k in [1, 5, 10, 20, 50] {
        group.bench_with_input(BenchmarkId::new("recall@k", k), &k, |bench, &k| {
            bench.iter(|| {
                let mut total_recall = 0.0;

                for query in &queries {
                    let f32_results = index.search(query, k, &storage).unwrap();
                    let f32_ids: HashSet<u64> = f32_results.iter().map(|r| r.vector_id.0).collect();

                    let bq_results = index.search_bq_rescored(query, k, 5, &storage).unwrap();
                    let bq_ids: HashSet<u64> = bq_results.iter().map(|(id, _)| id.0).collect();

                    total_recall += compute_recall(&f32_ids, &bq_ids, k);
                }

                total_recall / num_queries as f64
            })
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_recall_by_rescore_factor,
    bench_recall_bq_vs_rescored,
    bench_recall_by_index_size,
    bench_recall_at_k,
);
criterion_main!(benches);
