//! Benchmarks for HNSW Insertion Performance.
//!
//! Run with: `cargo bench --bench insert_bench`
//!
//! # Reproducibility
//!
//! All benchmarks use:
//! - Seed: 42 for RNG
//! - Dimensions: 128
//! - Distribution: Uniform [-1, 1]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::storage::VectorStorage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Generates deterministic test vectors.
fn generate_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect()
}

/// Benchmark: Insert throughput
fn bench_insert_throughput(c: &mut Criterion) {
    let dims = 128;
    let counts = [1_000, 10_000];
    let seed = 42;

    let mut group = c.benchmark_group("insert_throughput");

    for count in counts {
        group.throughput(Throughput::Elements(count as u64));
        // Increase sample size for stability, though build is slow
        group.sample_size(10);

        let vectors = generate_vectors(count, dims, seed);

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, &_count| {
            b.iter(|| {
                let config = HnswConfig::new(dims as u32);
                let mut storage = VectorStorage::new(&config, None);
                let mut index = HnswIndex::new(config, &storage).unwrap();

                for v in &vectors {
                    index.insert(black_box(v), &mut storage).unwrap();
                }
                black_box(index)
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_insert_throughput);
criterion_main!(benches);
