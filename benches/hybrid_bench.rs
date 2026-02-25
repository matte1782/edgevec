//! Benchmarks for hybrid search functionality.
//!
//! Performance targets (Week 39 Day 6):
//! - Sparse search (10k, k=100): <20ms
//! - RRF fusion (1k results): <2ms
//! - Linear fusion (1k results): <2ms
//! - Hybrid search (10k, k=10): <50ms

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::hint::black_box;

use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::hybrid::{linear_fusion, rrf_fusion, HybridSearchConfig, HybridSearcher};
use edgevec::sparse::{SparseSearcher, SparseStorage, SparseVector};
use edgevec::storage::VectorStorage;

// =============================================================================
// TEST DATA GENERATION
// =============================================================================

/// Generate test data with aligned dense and sparse vectors.
fn generate_test_data(
    num_vectors: usize,
    dense_dim: usize,
    sparse_nnz: usize,
    sparse_dim: u32,
) -> (HnswIndex, VectorStorage, SparseStorage) {
    let config = HnswConfig::new(dense_dim as u32);
    let mut dense_storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &dense_storage).expect("failed to create index");
    let mut sparse_storage = SparseStorage::new();

    // LCG for reproducible random data
    let mut seed: u64 = 42;
    let lcg = |s: &mut u64| -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    };

    for i in 0..num_vectors {
        // Dense vector
        let dense: Vec<f32> = (0..dense_dim)
            .map(|_| (lcg(&mut seed) % 1000) as f32 / 1000.0)
            .collect();
        let _id = index
            .insert(&dense, &mut dense_storage)
            .expect("insert failed");

        // Sparse vector - indices spread across vocabulary
        let indices: Vec<u32> = (0..sparse_nnz)
            .map(|j| ((i * sparse_nnz + j) % (sparse_dim as usize)) as u32)
            .collect();
        let mut indices_sorted = indices;
        indices_sorted.sort_unstable();
        indices_sorted.dedup();

        let values: Vec<f32> = indices_sorted
            .iter()
            .map(|_| (lcg(&mut seed) % 100) as f32 / 10.0)
            .collect();

        if !indices_sorted.is_empty() {
            let sparse = SparseVector::new(indices_sorted, values, sparse_dim).unwrap();
            sparse_storage
                .insert(&sparse)
                .expect("sparse insert failed");
        }
    }

    (index, dense_storage, sparse_storage)
}

fn random_dense_query(dim: usize, seed: u64) -> Vec<f32> {
    let mut s = seed;
    let lcg = |s: &mut u64| -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    };

    (0..dim)
        .map(|_| (lcg(&mut s) % 1000) as f32 / 1000.0)
        .collect()
}

fn random_sparse_query(nnz: usize, dim: u32, seed: u64) -> SparseVector {
    let mut s = seed;
    let lcg = |s: &mut u64| -> u64 {
        *s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
        *s
    };

    let mut indices: Vec<u32> = (0..nnz)
        .map(|_| (lcg(&mut s) % dim as u64) as u32)
        .collect();
    indices.sort_unstable();
    indices.dedup();

    let values: Vec<f32> = indices
        .iter()
        .map(|_| (lcg(&mut s) % 100) as f32 / 10.0)
        .collect();

    SparseVector::new(indices, values, dim).unwrap()
}

// =============================================================================
// SPARSE SEARCH BENCHMARKS
// =============================================================================

fn bench_sparse_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_search");
    group.sample_size(50);

    for num_vectors in [1_000, 10_000] {
        let (_, _, sparse_storage) = generate_test_data(num_vectors, 64, 50, 10_000);

        group.bench_with_input(
            BenchmarkId::new("brute_force", num_vectors),
            &num_vectors,
            |b, _| {
                let searcher = SparseSearcher::new(&sparse_storage);
                let query = random_sparse_query(50, 10_000, 12345);

                b.iter(|| black_box(searcher.search(&query, 100)));
            },
        );
    }

    group.finish();
}

// =============================================================================
// FUSION BENCHMARKS
// =============================================================================

fn bench_rrf_fusion(c: &mut Criterion) {
    let mut group = c.benchmark_group("rrf_fusion");
    group.sample_size(100);

    for list_size in [100, 500, 1000] {
        // Generate mock results
        let dense: Vec<(u64, f32)> = (0..list_size as u64)
            .map(|i| (i, 1.0 - (i as f32 / list_size as f32)))
            .collect();
        let sparse: Vec<(u64, f32)> = ((list_size / 2) as u64..(list_size * 3 / 2) as u64)
            .map(|i| {
                (
                    i,
                    10.0 - ((i - list_size as u64 / 2) as f32 / list_size as f32),
                )
            })
            .collect();

        group.bench_with_input(BenchmarkId::new("k60", list_size), &list_size, |b, _| {
            b.iter(|| black_box(rrf_fusion(&dense, &sparse, 60, 100)));
        });
    }

    group.finish();
}

fn bench_linear_fusion(c: &mut Criterion) {
    let mut group = c.benchmark_group("linear_fusion");
    group.sample_size(100);

    for list_size in [100, 500, 1000] {
        let dense: Vec<(u64, f32)> = (0..list_size as u64)
            .map(|i| (i, 1.0 - (i as f32 / list_size as f32)))
            .collect();
        let sparse: Vec<(u64, f32)> = ((list_size / 2) as u64..(list_size * 3 / 2) as u64)
            .map(|i| {
                (
                    i,
                    10.0 - ((i - list_size as u64 / 2) as f32 / list_size as f32),
                )
            })
            .collect();

        group.bench_with_input(
            BenchmarkId::new("alpha_0.5", list_size),
            &list_size,
            |b, _| {
                b.iter(|| black_box(linear_fusion(&dense, &sparse, 0.5, 100)));
            },
        );
    }

    group.finish();
}

// =============================================================================
// HYBRID SEARCH BENCHMARKS
// =============================================================================

fn bench_hybrid_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("hybrid_search");
    group.sample_size(30); // Fewer samples for slower benchmarks

    for num_vectors in [1_000, 10_000] {
        let (index, dense_storage, sparse_storage) =
            generate_test_data(num_vectors, 64, 50, 10_000);

        let dense_query = random_dense_query(64, 54321);
        let sparse_query = random_sparse_query(50, 10_000, 67890);

        group.bench_with_input(
            BenchmarkId::new("rrf", num_vectors),
            &num_vectors,
            |b, _| {
                let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);
                let config = HybridSearchConfig::rrf(20, 20, 10);

                b.iter(|| {
                    black_box(
                        searcher
                            .search(&dense_query, &sparse_query, &config)
                            .unwrap(),
                    )
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("linear", num_vectors),
            &num_vectors,
            |b, _| {
                let searcher = HybridSearcher::new(&index, &dense_storage, &sparse_storage);
                let config = HybridSearchConfig::linear(20, 20, 10, 0.5);

                b.iter(|| {
                    black_box(
                        searcher
                            .search(&dense_query, &sparse_query, &config)
                            .unwrap(),
                    )
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// CRITERION MAIN
// =============================================================================

criterion_group!(
    benches,
    bench_sparse_search,
    bench_rrf_fusion,
    bench_linear_fusion,
    bench_hybrid_search,
);

criterion_main!(benches);
