//! Benchmarks for HNSW Search Performance.
//!
//! Run with: `cargo bench --bench search_bench`
//!
//! # Reproducibility
//!
//! All benchmarks use:
//! - Seed: 42 for RNG
//! - Dimensions: 128
//! - Distribution: Uniform [-1, 1]

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::hnsw::{HnswConfig, HnswIndex, SearchContext};
use edgevec::storage::VectorStorage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::hint::black_box;

/// Generates deterministic test vectors.
fn generate_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect()
}

/// Benchmark: Search latency vs vector count
///
/// Measures latency for search operations at different scales.
fn bench_search_latency(c: &mut Criterion) {
    let dims = 128;
    let k = 10;
    let seed = 42;
    let counts = [1_000, 10_000];

    let mut group = c.benchmark_group("search_latency");

    for count in counts {
        // Setup Index once per count
        let vectors = generate_vectors(count, dims, seed);
        let config = HnswConfig::new(dims as u32);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).unwrap();

        for v in &vectors {
            index.insert(v, &mut storage).unwrap();
        }

        // Use the first vector as a query (guaranteed to be in distribution)
        let query = &vectors[0];

        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            let mut search_ctx = SearchContext::new();
            b.iter(|| {
                black_box(
                    index
                        .search_with_context(black_box(query), k, &storage, &mut search_ctx)
                        .unwrap(),
                )
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_search_latency);
criterion_main!(benches);
