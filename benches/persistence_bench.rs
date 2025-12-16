use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::hnsw::{HnswConfig, HnswIndex};
use edgevec::persistence::storage::MemoryBackend;
use edgevec::persistence::{read_snapshot, write_snapshot};
use edgevec::storage::VectorStorage;
use rand::Rng;

fn generate_vectors(count: usize, dim: usize) -> Vec<Vec<f32>> {
    let mut rng = rand::thread_rng();
    (0..count)
        .map(|_| (0..dim).map(|_| rng.gen()).collect())
        .collect()
}

fn bench_snapshot_persistence(c: &mut Criterion) {
    let dim = 128;
    // Benchmarking 100k is ideal but might be slow for repeated runs.
    // We'll use 10k and 50k as proxies, and extrapolated or run 100k if fast enough.
    // Given the request for 100k target, we will try to run 100k directly.
    let counts = [10_000, 50_000, 100_000];

    let mut group = c.benchmark_group("snapshot_persistence");
    group.sample_size(10); // Reduce sample size for large I/O benchmarks

    for count in counts {
        // Setup Data
        let config = HnswConfig::new(dim as u32);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

        let vectors = generate_vectors(count, dim);
        for vec in &vectors {
            let id = storage.insert(vec).unwrap();
            index.add_node(id, 0).unwrap();
        }

        // 1. Benchmark Save
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(BenchmarkId::new("save", count), &count, |b, &_| {
            b.iter(|| {
                let mut backend = MemoryBackend::new();
                write_snapshot(&index, &storage, &mut backend).unwrap();
            })
        });

        // 2. Benchmark Load
        // Prepare a backend with data
        let mut backend = MemoryBackend::new();
        write_snapshot(&index, &storage, &mut backend).unwrap();

        group.bench_with_input(BenchmarkId::new("load", count), &count, |b, &_| {
            b.iter(|| {
                read_snapshot(&backend).unwrap();
            })
        });
    }
    group.finish();
}

criterion_group!(benches, bench_snapshot_persistence);
criterion_main!(benches);
