//! Benchmarks for Soft Delete overhead in EdgeVec.
//!
//! Measures search latency impact when 50% of vectors are deleted.
//!
//! Run with: `cargo bench --bench delete_bench`

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
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

/// Helper to build index
fn build_index(vectors: &[Vec<f32>], dims: u32) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(dims);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    for v in vectors {
        index.insert(v, &mut storage).unwrap();
    }
    (index, storage)
}

fn bench_delete_impact(c: &mut Criterion) {
    let dims = 32; // Small dims to focus on graph traversal overhead
    let count = 10_000;
    let seed = 42;
    let k = 10;

    // 1. Generate data
    let vectors = generate_vectors(count, dims as usize, seed);
    let query = &vectors[0];

    // 2. Prepare 0% Deleted (Baseline)
    let (index_clean, storage_clean) = build_index(&vectors, dims);

    // 3. Prepare 50% Deleted
    let (mut index_dirty, storage_dirty) = build_index(&vectors, dims);
    let mut deleted_count = 0;
    // Delete even IDs
    for i in 0..count {
        if i % 2 == 0 {
            let id = VectorId((i + 1) as u64);
            let _ = index_dirty.soft_delete(id);
            deleted_count += 1;
        }
    }
    assert_eq!(deleted_count, count / 2);

    let mut group = c.benchmark_group("delete_overhead");
    group.throughput(Throughput::Elements(1));

    // Benchmark Clean
    group.bench_function("search_0_percent_deleted", |b| {
        b.iter(|| {
            black_box(
                index_clean
                    .search(black_box(query), k, &storage_clean)
                    .unwrap(),
            )
        });
    });

    // Benchmark Dirty
    group.bench_function("search_50_percent_deleted", |b| {
        b.iter(|| {
            black_box(
                index_dirty
                    .search(black_box(query), k, &storage_dirty)
                    .unwrap(),
            )
        });
    });

    group.finish();
}

criterion_group!(benches, bench_delete_impact);
criterion_main!(benches);
