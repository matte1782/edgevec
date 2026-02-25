//! Benchmarks for EdgeVec distance metrics.
//!
//! Run with: `cargo bench`
//!
//! # Reproducibility
//!
//! All benchmarks use:
//! - Seed: 42 for RNG
//! - Dimensions: 128, 384, 768, 1536
//! - Distribution: Uniform [-1, 1]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use edgevec::metric::{DotProduct, L2Squared, Metric};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Generates deterministic test vectors.
fn generate_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect()
}

/// Benchmark: L2 Squared Distance
fn bench_l2_squared(c: &mut Criterion) {
    let seed = 42;
    let mut group = c.benchmark_group("l2_squared");

    for dims in [128, 384, 768, 1536] {
        let vectors = generate_vectors(2, dims, seed);
        let a = &vectors[0];
        let b = &vectors[1];

        group.throughput(Throughput::Elements(dims as u64));
        group.bench_with_input(BenchmarkId::from_parameter(dims), &dims, |bencher, _| {
            bencher.iter(|| L2Squared::distance(black_box(a), black_box(b)));
        });
    }
    group.finish();
}

/// Benchmark: Dot Product
fn bench_dot_product(c: &mut Criterion) {
    let seed = 42;
    let mut group = c.benchmark_group("dot_product");

    for dims in [128, 384, 768, 1536] {
        let vectors = generate_vectors(2, dims, seed);
        let a = &vectors[0];
        let b = &vectors[1];

        group.throughput(Throughput::Elements(dims as u64));
        group.bench_with_input(BenchmarkId::from_parameter(dims), &dims, |bencher, _| {
            bencher.iter(|| DotProduct::distance(black_box(a), black_box(b)));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_l2_squared, bench_dot_product);
criterion_main!(benches);
