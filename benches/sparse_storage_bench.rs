//! Benchmarks for sparse storage operations.
//!
//! Performance targets from RFC-007:
//! - Insert: P50 <50us, P99 <100us
//! - Get: <1us
//! - Iteration: <100ms for 100k vectors

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::sparse::{SparseId, SparseStorage, SparseVector};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;
use std::hint::black_box;
use std::time::Duration;

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

    // Generate random non-zero values
    let values: Vec<f32> = (0..nnz)
        .map(|_| {
            let mut v = rng.gen_range(-1.0..1.0);
            // Avoid exact zero
            if v == 0.0 {
                v = 0.001;
            }
            v
        })
        .collect();

    SparseVector::new(indices, values, dim).expect("Generated vector should be valid")
}

/// Create storage pre-populated with N sparse vectors.
fn create_storage(count: usize, dim: u32, nnz: usize) -> SparseStorage {
    let mut storage = SparseStorage::new();
    for i in 0..count {
        let vec = random_sparse(dim, nnz, i as u64);
        storage.insert(&vec).expect("Insert should succeed");
    }
    storage
}

// =============================================================================
// BENCHMARK GROUP: storage_insert
// =============================================================================

/// Benchmark insert at various batch sizes.
fn bench_storage_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_insert");

    // Configure for accurate P99 measurement
    group.sample_size(500);
    group.measurement_time(Duration::from_secs(10));

    let dim = 10_000u32;
    let nnz = 50usize;

    // Insert single vector
    group.throughput(Throughput::Elements(1));
    group.bench_function("single", |b| {
        let mut storage = SparseStorage::new();
        let vec = random_sparse(dim, nnz, 42);

        b.iter(|| storage.insert(black_box(&vec)).unwrap());
    });

    // Insert into storage with existing vectors (simulates steady state)
    for initial_count in [100, 1000, 10000] {
        group.bench_with_input(
            BenchmarkId::new("into_existing", initial_count),
            &initial_count,
            |b, &count| {
                let mut storage = create_storage(count, dim, nnz);
                let vec = random_sparse(dim, nnz, 999);

                b.iter(|| storage.insert(black_box(&vec)).unwrap());
            },
        );
    }

    // Batch insert throughput
    for batch_size in [100, 1000, 10000] {
        group.throughput(Throughput::Elements(batch_size as u64));
        group.bench_with_input(
            BenchmarkId::new("batch", batch_size),
            &batch_size,
            |b, &count| {
                let vectors: Vec<SparseVector> = (0..count)
                    .map(|i| random_sparse(dim, nnz, i as u64))
                    .collect();

                b.iter(|| {
                    let mut storage = SparseStorage::new();
                    for vec in &vectors {
                        storage.insert(black_box(vec)).unwrap();
                    }
                    storage
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_get
// =============================================================================

/// Benchmark get operations.
fn bench_storage_get(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_get");

    group.sample_size(1000);
    group.measurement_time(Duration::from_secs(5));

    let dim = 10_000u32;
    let nnz = 50usize;

    // Get from 10k storage (matches RFC-007 target scenario)
    let storage = create_storage(10_000, dim, nnz);

    // Get random vector
    group.bench_function("random_from_10k", |b| {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        b.iter(|| {
            let id = SparseId::new(rng.gen_range(0..10_000u64));
            storage.get(black_box(id))
        });
    });

    // Get first vector (cache-friendly)
    group.bench_function("first_from_10k", |b| {
        let id = SparseId::new(0);
        b.iter(|| storage.get(black_box(id)));
    });

    // Get last vector (potential cache-miss)
    group.bench_function("last_from_10k", |b| {
        let id = SparseId::new(9999);
        b.iter(|| storage.get(black_box(id)));
    });

    // Get non-existent (should return None quickly)
    group.bench_function("missing_from_10k", |b| {
        let id = SparseId::new(99999);
        b.iter(|| storage.get(black_box(id)));
    });

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_iter
// =============================================================================

/// Benchmark iteration over storage.
fn bench_storage_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_iter");

    group.sample_size(100);
    group.measurement_time(Duration::from_secs(10));

    let dim = 10_000u32;
    let nnz = 50usize;

    // Iterate over various storage sizes
    for count in [1000, 10000, 100000] {
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("count_{count}")),
            &count,
            |b, &count| {
                let storage = create_storage(count, dim, nnz);

                b.iter(|| {
                    let mut sum = 0.0f32;
                    for (id, vec) in storage.iter() {
                        // Access both id and vector to ensure full iteration
                        sum += black_box(id.as_u64()) as f32 + vec.nnz() as f32;
                    }
                    sum
                });
            },
        );
    }

    // Iterate with 10% deletions
    let mut storage_with_deletions = create_storage(10_000, dim, nnz);
    // Delete every 10th vector
    for i in (0..10_000).step_by(10) {
        let _ = storage_with_deletions.delete(SparseId::new(i));
    }

    group.bench_function("10k_with_deletions", |b| {
        b.iter(|| {
            let mut sum = 0.0f32;
            for (id, vec) in storage_with_deletions.iter() {
                sum += black_box(id.as_u64()) as f32 + vec.nnz() as f32;
            }
            sum
        });
    });

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_roundtrip
// =============================================================================

/// Benchmark serialization roundtrip.
fn bench_storage_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_roundtrip");

    group.sample_size(50);
    group.measurement_time(Duration::from_secs(15));

    let dim = 10_000u32;
    let nnz = 50usize;

    // Roundtrip at various sizes
    for count in [1000, 10000] {
        group.throughput(Throughput::Elements(count as u64));

        // Serialize benchmark
        group.bench_with_input(BenchmarkId::new("serialize", count), &count, |b, &count| {
            let storage = create_storage(count, dim, nnz);

            b.iter(|| storage.to_bytes());
        });

        // Deserialize benchmark
        let storage = create_storage(count, dim, nnz);
        let bytes = storage.to_bytes();

        group.bench_with_input(
            BenchmarkId::new("deserialize", count),
            &bytes,
            |b, bytes| {
                b.iter(|| SparseStorage::from_bytes(black_box(bytes)));
            },
        );

        // Full roundtrip
        group.bench_with_input(
            BenchmarkId::new("full_roundtrip", count),
            &count,
            |b, &count| {
                let storage = create_storage(count, dim, nnz);

                b.iter(|| {
                    let bytes = storage.to_bytes();
                    SparseStorage::from_bytes(black_box(&bytes))
                });
            },
        );
    }

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_delete
// =============================================================================

/// Benchmark delete operations.
fn bench_storage_delete(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_delete");

    group.sample_size(500);

    let dim = 10_000u32;
    let nnz = 50usize;

    // Single delete
    group.bench_function("single", |b| {
        b.iter_batched(
            || create_storage(1000, dim, nnz),
            |mut storage| storage.delete(black_box(SparseId::new(500))).unwrap(),
            criterion::BatchSize::SmallInput,
        );
    });

    // Batch delete (delete 10% of vectors)
    group.bench_function("batch_10_percent", |b| {
        b.iter_batched(
            || create_storage(10_000, dim, nnz),
            |mut storage| {
                for i in (0..10_000).step_by(10) {
                    let _ = storage.delete(SparseId::new(i));
                }
            },
            criterion::BatchSize::LargeInput,
        );
    });

    group.finish();
}

// =============================================================================
// BENCHMARK GROUP: storage_memory
// =============================================================================

/// Benchmark memory allocation patterns.
fn bench_storage_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_memory");

    group.sample_size(100);

    let dim = 10_000u32;

    // Memory with varying sparsity
    for nnz in [10, 50, 100, 200] {
        group.bench_with_input(BenchmarkId::new("create_10k", nnz), &nnz, |b, &nnz| {
            b.iter(|| create_storage(10_000, dim, nnz));
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_storage_insert,
    bench_storage_get,
    bench_storage_iter,
    bench_storage_roundtrip,
    bench_storage_delete,
    bench_storage_memory
);
criterion_main!(benches);
