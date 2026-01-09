//! Benchmarks for sparse vector operations.
//!
//! Performance targets from RFC-007:
//! - Dot product (50 nnz): P50 <300ns, P99 <500ns
//! - Dot product (100 nnz): P50 <600ns, P99 <1us

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use edgevec::sparse::{sparse_cosine, sparse_dot_product, sparse_norm, SparseVector};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

/// Generate a random sparse vector with given parameters.
///
/// Uses deterministic RNG for reproducible benchmarks.
fn random_sparse(dim: u32, nnz: usize, seed: u64) -> SparseVector {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // Generate unique sorted indices
    let mut indices: Vec<u32> = (0..dim).collect();
    indices.shuffle(&mut rng);
    indices.truncate(nnz);
    indices.sort();

    // Generate random values
    let values: Vec<f32> = (0..nnz).map(|_| rng.gen_range(-1.0..1.0)).collect();

    SparseVector::new(indices, values, dim).expect("Generated vector should be valid")
}

/// Benchmark dot product at various sparsity levels.
fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_dot_product");

    // Configure for accurate P99 measurement
    group.sample_size(1000);
    group.measurement_time(std::time::Duration::from_secs(10));

    let dim = 10_000u32;

    for nnz in [10, 50, 100, 200, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("nnz_{nnz}")),
            &nnz,
            |b, &nnz| {
                let a = random_sparse(dim, nnz, 42);
                let query = random_sparse(dim, nnz, 123);

                b.iter(|| sparse_dot_product(black_box(&a), black_box(&query)));
            },
        );
    }

    group.finish();
}

/// Benchmark dot product with varying overlap.
fn bench_dot_product_overlap(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_dot_overlap");
    group.sample_size(500);

    let dim = 10_000u32;
    let nnz = 100usize;

    // No overlap (different index ranges)
    group.bench_function("no_overlap", |b| {
        let a = SparseVector::new((0..nnz as u32).collect(), vec![1.0; nnz], dim).unwrap();
        let query =
            SparseVector::new((5000..(5000 + nnz as u32)).collect(), vec![1.0; nnz], dim).unwrap();

        b.iter(|| sparse_dot_product(black_box(&a), black_box(&query)));
    });

    // Full overlap (same indices)
    group.bench_function("full_overlap", |b| {
        let a = random_sparse(dim, nnz, 42);
        let query = SparseVector::new(a.indices().to_vec(), vec![1.0; nnz], dim).unwrap();

        b.iter(|| sparse_dot_product(black_box(&a), black_box(&query)));
    });

    // Partial overlap (~50%)
    group.bench_function("partial_overlap", |b| {
        let a = random_sparse(dim, nnz, 42);
        let query = random_sparse(dim, nnz, 43); // Different seed, ~50% overlap expected

        b.iter(|| sparse_dot_product(black_box(&a), black_box(&query)));
    });

    group.finish();
}

/// Benchmark cosine similarity.
fn bench_cosine(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_cosine");
    group.sample_size(500);

    let dim = 10_000u32;

    for nnz in [50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("nnz_{nnz}")),
            &nnz,
            |b, &nnz| {
                let a = random_sparse(dim, nnz, 42);
                let query = random_sparse(dim, nnz, 123);

                b.iter(|| sparse_cosine(black_box(&a), black_box(&query)));
            },
        );
    }

    group.finish();
}

/// Benchmark norm calculation.
fn bench_norm(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_norm");
    group.sample_size(500);

    let dim = 10_000u32;

    for nnz in [50, 100, 200, 500] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("nnz_{nnz}")),
            &nnz,
            |b, &nnz| {
                let v = random_sparse(dim, nnz, 42);

                b.iter(|| sparse_norm(black_box(&v)));
            },
        );
    }

    group.finish();
}

/// Benchmark vector construction.
fn bench_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_construction");
    group.sample_size(500);

    let dim = 10_000u32;

    // Benchmark new() with pre-sorted data
    for nnz in [50, 100, 200] {
        let indices: Vec<u32> = (0..nnz as u32 * 100).step_by(100).collect();
        let values: Vec<f32> = vec![1.0; nnz];

        group.bench_with_input(
            BenchmarkId::new("new", nnz),
            &(indices.clone(), values.clone(), dim),
            |b, (i, v, d)| {
                b.iter(|| {
                    SparseVector::new(black_box(i.clone()), black_box(v.clone()), black_box(*d))
                });
            },
        );
    }

    // Benchmark from_pairs() with unsorted data
    for nnz in [50, 100, 200] {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let mut indices: Vec<u32> = (0..dim).collect();
        indices.shuffle(&mut rng);
        indices.truncate(nnz);
        // Don't sort - let from_pairs do it

        let pairs: Vec<(u32, f32)> = indices
            .iter()
            .map(|&i| (i, rng.gen_range(-1.0..1.0)))
            .collect();

        group.bench_with_input(
            BenchmarkId::new("from_pairs", nnz),
            &(pairs.clone(), dim),
            |b, (p, d)| {
                b.iter(|| SparseVector::from_pairs(black_box(p), black_box(*d)));
            },
        );
    }

    group.finish();
}

/// Benchmark normalization.
fn bench_normalize(c: &mut Criterion) {
    let mut group = c.benchmark_group("sparse_normalize");
    group.sample_size(500);

    let dim = 10_000u32;

    for nnz in [50, 100, 200] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("nnz_{nnz}")),
            &nnz,
            |b, &nnz| {
                let v = random_sparse(dim, nnz, 42);

                b.iter(|| v.normalize());
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_dot_product,
    bench_dot_product_overlap,
    bench_cosine,
    bench_norm,
    bench_construction,
    bench_normalize
);
criterion_main!(benches);
