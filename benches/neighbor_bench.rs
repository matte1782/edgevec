use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use std::hint::black_box;
use edgevec::hnsw::neighbor::NeighborPool;

fn bench_neighbor_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("neighbor_ops");
    let mut pool = NeighborPool::new();

    // 1. Encode Benchmarks
    // Scenario: Node with 32 neighbors (M0=32)
    // Neighbors are sorted u32s with some spread.
    let neighbors: Vec<u32> = (0..32).map(|i| i * 100).collect();

    group.throughput(Throughput::Elements(32));
    group.bench_function("encode_vbyte_32", |b| {
        b.iter(|| NeighborPool::encode_neighbors(&neighbors))
    });

    // 2. Decode Benchmarks
    let encoded = NeighborPool::encode_neighbors(&neighbors);

    group.throughput(Throughput::Elements(32));
    group.bench_function("decode_vbyte_32", |b| {
        b.iter(|| NeighborPool::decode_neighbors(&encoded))
    });

    // 3. Alloc/Free Cycle
    // Alloc 64 bytes -> Free 64 bytes
    // This measures the overhead of BTreeMap free list operations.
    group.throughput(Throughput::Elements(1));
    group.bench_function("alloc_free_cycle_64B", |b| {
        b.iter(|| {
            let (off, cap) = pool.alloc(64).unwrap();
            pool.free(off, cap);
        })
    });

    group.finish();
}

criterion_group!(benches, bench_neighbor_operations);
criterion_main!(benches);
