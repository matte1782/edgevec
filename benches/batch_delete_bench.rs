//! [C1 FIX] Batch Delete Performance Benchmark (AC18.4.5)
//!
//! Validates that batch deletion is no slower than N individual calls.
//! Expected: Batch should be comparable or faster due to deduplication overhead
//! being amortized over many operations.

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use edgevec::hnsw::{HnswConfig, HnswIndex, VectorId};
use edgevec::storage::VectorStorage;
use std::hint::black_box;

fn create_test_index(count: usize) -> (HnswIndex, VectorStorage) {
    let config = HnswConfig::new(128);
    let mut storage = VectorStorage::new(&config, None);
    let mut index = HnswIndex::new(config, &storage).unwrap();

    for i in 0..count {
        let vec: Vec<f32> = (0..128).map(|j| (i * 128 + j) as f32).collect();
        index.insert(&vec, &mut storage).unwrap();
    }

    (index, storage)
}

fn bench_individual_deletes(c: &mut Criterion) {
    c.bench_function("batch_delete/individual_100", |b| {
        b.iter_batched(
            || {
                let (index, storage) = create_test_index(1000);
                let ids: Vec<VectorId> = (1..=100).map(VectorId).collect();
                (index, storage, ids)
            },
            |(mut index, _storage, ids)| {
                for id in ids {
                    black_box(index.soft_delete(id).unwrap());
                }
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_batch_delete_100(c: &mut Criterion) {
    c.bench_function("batch_delete/batch_100", |b| {
        b.iter_batched(
            || {
                let (index, storage) = create_test_index(1000);
                let ids: Vec<VectorId> = (1..=100).map(VectorId).collect();
                (index, storage, ids)
            },
            |(mut index, _storage, ids)| {
                black_box(index.soft_delete_batch(&ids));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_batch_delete_1k(c: &mut Criterion) {
    c.bench_function("batch_delete/batch_1k", |b| {
        b.iter_batched(
            || {
                let (index, storage) = create_test_index(10000);
                let ids: Vec<VectorId> = (1..=1000).map(VectorId).collect();
                (index, storage, ids)
            },
            |(mut index, _storage, ids)| {
                black_box(index.soft_delete_batch(&ids));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_batch_delete_with_duplicates(c: &mut Criterion) {
    c.bench_function("batch_delete/batch_100_with_50pct_duplicates", |b| {
        b.iter_batched(
            || {
                let (index, storage) = create_test_index(1000);
                // 100 IDs, but only 50 unique (each appears twice)
                let mut ids: Vec<VectorId> = (1..=50).map(VectorId).collect();
                ids.extend((1..=50).map(VectorId));
                (index, storage, ids)
            },
            |(mut index, _storage, ids)| {
                black_box(index.soft_delete_batch(&ids));
            },
            BatchSize::SmallInput,
        );
    });
}

fn bench_batch_delete_with_progress(c: &mut Criterion) {
    c.bench_function("batch_delete/batch_100_with_progress", |b| {
        b.iter_batched(
            || {
                let (index, storage) = create_test_index(1000);
                let ids: Vec<VectorId> = (1..=100).map(VectorId).collect();
                (index, storage, ids)
            },
            |(mut index, _storage, ids)| {
                black_box(index.soft_delete_batch_with_progress(&ids, |_, _| {}));
            },
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_individual_deletes,
    bench_batch_delete_100,
    bench_batch_delete_1k,
    bench_batch_delete_with_duplicates,
    bench_batch_delete_with_progress
);
criterion_main!(benches);
