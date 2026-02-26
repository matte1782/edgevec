//! Benchmarks for HNSW initialization and memory usage.
//!
//! Run with: `cargo bench --bench hnsw_init_bench`

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::storage::VectorStorage;

/// Benchmark: HNSW Initialization Latency
///
/// Measures the time to create an empty HnswIndex.
fn bench_hnsw_init(c: &mut Criterion) {
    let mut group = c.benchmark_group("hnsw_init");

    group.bench_function("init_empty_index", |b| {
        b.iter(|| {
            let config = HnswConfig::new(black_box(128));
            // We need a storage to initialize the index
            let storage = VectorStorage::new(&config, None);
            let _index = HnswIndex::new(config, &storage).unwrap();
        });
    });

    group.finish();
}

criterion_group!(benches, bench_hnsw_init);
criterion_main!(benches);
