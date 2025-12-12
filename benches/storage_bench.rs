//! Benchmarks for VectorStorage insert performance.
//!
//! Run with: `cargo bench --bench storage_bench`

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use edgevec::hnsw::HnswConfig;
use edgevec::persistence::storage::file::FileBackend;
use edgevec::persistence::wal::WalAppender;
use edgevec::storage::VectorStorage;
use tempfile::NamedTempFile;

fn bench_storage_insert(c: &mut Criterion) {
    let mut group = c.benchmark_group("storage_insert");
    let config = HnswConfig::new(128); // 128 dimensions

    // Setup vector for insertion
    let vector = vec![1.0f32; 128];

    // Scenario A: In-Memory (No WAL)
    group.throughput(Throughput::Elements(1));
    group.bench_function("insert_memory_only", |b| {
        // We need to re-create storage or reset it to avoid growing infinitely during bench loop,
        // or we just measure the insert into an existing storage.
        // Inserting into a `Vec` is amortized O(1).
        // Let's create a storage and insert many times.
        // To avoid allocation noise, we might pre-reserve, but standard usage is dynamic.
        // However, Criterion runs many iterations.
        // If we keep inserting, the storage grows.
        // We want to measure the latency of a single insert.
        // We can setup a fresh storage for each batch if possible, or just keep inserting.
        // Let's reuse one storage instance to simulate steady state.
        let mut storage = VectorStorage::new(&config, None);
        b.iter(|| {
            black_box(storage.insert(black_box(&vector))).unwrap();
        });
    });

    // Scenario B: With WAL (Durable)
    group.bench_function("insert_with_wal", |b| {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_owned();
        // We clone path because we re-open file in loop?
        // No, WAL appender holds the file writer.
        // We need a single storage with WAL.

        // We need to be careful: if we keep inserting, the WAL grows on disk.
        // IO performance might degrade if file gets huge, but for microbenchmark it's probably fine.
        // We want to measure the sync overhead.

        let backend = FileBackend::new(&path);
        let wal = WalAppender::new(Box::new(backend), 0);
        let mut storage = VectorStorage::new(&config, Some(wal));

        b.iter(|| {
            black_box(storage.insert(black_box(&vector))).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_storage_insert);
criterion_main!(benches);
