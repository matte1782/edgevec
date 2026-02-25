//! Benchmarks for BQ vs F32 Search Performance.
//!
//! Run with: `cargo bench --bench bq_search`
//!
//! ## Performance Targets (RFC-002)
//!
//! | Metric | Target |
//! |:-------|:-------|
//! | BQ search speedup vs F32 | 3-5x |
//! | BQ+rescore vs F32 | 1-2x |
//! | BQ memory reduction | 32x |
//!
//! ## Methodology
//!
//! - Index Size: 1K, 10K vectors
//! - Dimension: 128D (common embedding size)
//! - Queries: 100 per run (averaged)
//! - k: 10 (standard recall@10)

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::storage::VectorStorage;
use rand::{Rng, RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Generate a random f32 vector with values in [-1, 1].
fn generate_vector(dims: usize, seed: u64) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..dims).map(|_| rng.random_range(-1.0..1.0)).collect()
}

/// Create a BQ-enabled index with n random vectors.
fn create_bq_index(n: usize, dims: u32, seed: u64) -> (HnswIndex, VectorStorage) {
    let mut config = HnswConfig::new(dims);
    config.m = 16;
    config.m0 = 32;
    config.ef_construction = 100;
    config.ef_search = 64;

    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::with_bq(config, &storage).expect("BQ index creation failed");

    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    for _ in 0..n {
        let v: Vec<f32> = (0..dims).map(|_| rng.random_range(-1.0..1.0)).collect();
        index.insert_bq(&v, &mut storage).expect("Insert failed");
    }

    (index, storage)
}

// ============================================================================
// SEARCH LATENCY BENCHMARKS
// ============================================================================

/// Benchmark F32 vs BQ vs BQ+rescore search latency.
fn bench_search_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("search_comparison");

    let dims = 128u32;

    for n in [1_000, 10_000] {
        let (index, storage) = create_bq_index(n, dims, 42);

        // Pre-generate queries
        let queries: Vec<Vec<f32>> = (0..100)
            .map(|i| generate_vector(dims as usize, 1000 + i))
            .collect();

        group.throughput(Throughput::Elements(100));

        // F32 search
        group.bench_with_input(BenchmarkId::new("f32_search", n), &n, |bench, _| {
            bench.iter(|| {
                for q in &queries {
                    black_box(index.search(black_box(q), 10, &storage).unwrap());
                }
            })
        });

        // Raw BQ search
        group.bench_with_input(BenchmarkId::new("bq_search", n), &n, |bench, _| {
            bench.iter(|| {
                for q in &queries {
                    black_box(index.search_bq(black_box(q), 10, &storage).unwrap());
                }
            })
        });

        // BQ+rescore search (factor=3)
        group.bench_with_input(BenchmarkId::new("bq_rescored_3x", n), &n, |bench, _| {
            bench.iter(|| {
                for q in &queries {
                    black_box(
                        index
                            .search_bq_rescored(black_box(q), 10, 3, &storage)
                            .unwrap(),
                    );
                }
            })
        });

        // BQ+rescore search (factor=5)
        group.bench_with_input(BenchmarkId::new("bq_rescored_5x", n), &n, |bench, _| {
            bench.iter(|| {
                for q in &queries {
                    black_box(
                        index
                            .search_bq_rescored(black_box(q), 10, 5, &storage)
                            .unwrap(),
                    );
                }
            })
        });
    }

    group.finish();
}

/// Benchmark single-query latency for precise timing.
fn bench_single_query_latency(c: &mut Criterion) {
    let mut group = c.benchmark_group("single_query_latency");

    let dims = 128u32;
    let n = 10_000;
    let (index, storage) = create_bq_index(n, dims, 42);
    let query = generate_vector(dims as usize, 9999);

    group.throughput(Throughput::Elements(1));

    group.bench_function("f32_10k_128d", |bench| {
        bench.iter(|| black_box(index.search(black_box(&query), 10, &storage).unwrap()))
    });

    group.bench_function("bq_10k_128d", |bench| {
        bench.iter(|| black_box(index.search_bq(black_box(&query), 10, &storage).unwrap()))
    });

    group.bench_function("bq_rescored_10k_128d", |bench| {
        bench.iter(|| {
            black_box(
                index
                    .search_bq_rescored(black_box(&query), 10, 3, &storage)
                    .unwrap(),
            )
        })
    });

    group.finish();
}

/// Benchmark varying k values.
fn bench_varying_k(c: &mut Criterion) {
    let mut group = c.benchmark_group("varying_k");

    let dims = 128u32;
    let n = 10_000;
    let (index, storage) = create_bq_index(n, dims, 42);
    let query = generate_vector(dims as usize, 9999);

    for k in [1, 5, 10, 20, 50] {
        group.bench_with_input(BenchmarkId::new("f32", k), &k, |bench, &k| {
            bench.iter(|| black_box(index.search(black_box(&query), k, &storage).unwrap()))
        });

        group.bench_with_input(BenchmarkId::new("bq", k), &k, |bench, &k| {
            bench.iter(|| black_box(index.search_bq(black_box(&query), k, &storage).unwrap()))
        });

        group.bench_with_input(BenchmarkId::new("bq_rescored", k), &k, |bench, &k| {
            bench.iter(|| {
                black_box(
                    index
                        .search_bq_rescored(black_box(&query), k, 3, &storage)
                        .unwrap(),
                )
            })
        });
    }

    group.finish();
}

/// Benchmark varying dimensions.
fn bench_varying_dims(c: &mut Criterion) {
    let mut group = c.benchmark_group("varying_dims");

    let n = 5_000;

    for dims in [128u32, 256, 384, 512, 768] {
        let (index, storage) = create_bq_index(n, dims, 42);
        let query = generate_vector(dims as usize, 9999);

        group.bench_with_input(BenchmarkId::new("f32", dims), &dims, |bench, _| {
            bench.iter(|| black_box(index.search(black_box(&query), 10, &storage).unwrap()))
        });

        group.bench_with_input(BenchmarkId::new("bq", dims), &dims, |bench, _| {
            bench.iter(|| black_box(index.search_bq(black_box(&query), 10, &storage).unwrap()))
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_search_comparison,
    bench_single_query_latency,
    bench_varying_k,
    bench_varying_dims,
);
criterion_main!(benches);
