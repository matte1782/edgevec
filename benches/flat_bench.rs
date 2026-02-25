//! Benchmarks for FlatIndex Performance.
//!
//! Run with: `cargo bench --bench flat_bench`
//!
//! # Reproducibility
//!
//! All benchmarks use:
//! - Seed: 42 for RNG
//! - Dimensions: 128 (default), 768 (high-dim)
//! - Distribution: Uniform [-1, 1]
//!
//! # Performance Targets (Week 40 Day 3)
//!
//! - Insert: O(1) amortized, <1ms for single vector
//! - Search (10k @ 768D): <50ms
//! - BQ Search: ~10x faster than F32 search

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::index::{DistanceMetric, FlatIndex, FlatIndexConfig};
use rand::{Rng, RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Generates deterministic test vectors.
fn generate_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.random_range(-1.0..1.0)).collect())
        .collect()
}

/// Benchmark: Insert latency
///
/// Measures O(1) insert performance.
fn bench_insert_latency(c: &mut Criterion) {
    let dims = 128;
    let seed = 42;
    let vectors = generate_vectors(1000, dims, seed);

    let mut group = c.benchmark_group("flat_insert");
    group.throughput(Throughput::Elements(1));

    group.bench_function("single_128d", |b| {
        let mut index = FlatIndex::new(FlatIndexConfig::new(dims as u32));
        let mut i = 0;
        b.iter(|| {
            black_box(
                index
                    .insert(black_box(&vectors[i % vectors.len()]))
                    .unwrap(),
            );
            i += 1;
        });
    });

    group.finish();
}

/// Benchmark: Search latency vs vector count
///
/// Measures latency for brute-force search at different scales.
fn bench_search_latency(c: &mut Criterion) {
    let dims = 128;
    let k = 10;
    let seed = 42;
    let counts = [1_000, 5_000, 10_000];

    let mut group = c.benchmark_group("flat_search_128d");

    for count in counts {
        let vectors = generate_vectors(count, dims, seed);
        let config = FlatIndexConfig::new(dims as u32);
        let mut index = FlatIndex::new(config);

        for v in &vectors {
            index.insert(v).unwrap();
        }

        let query = &vectors[0];

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(index.search(black_box(query), k).unwrap()));
        });
    }

    group.finish();
}

/// Benchmark: High-dimension search (768D)
///
/// Tests the <50ms target for 10k vectors @ 768D.
fn bench_search_768d(c: &mut Criterion) {
    let dims = 768;
    let k = 10;
    let seed = 42;

    let mut group = c.benchmark_group("flat_search_768d");

    for count in [1_000, 5_000, 10_000] {
        let vectors = generate_vectors(count, dims, seed);
        let config = FlatIndexConfig::new(dims as u32);
        let mut index = FlatIndex::new(config);

        for v in &vectors {
            index.insert(v).unwrap();
        }

        let query = &vectors[0];

        group.throughput(Throughput::Elements(1));
        group.bench_with_input(BenchmarkId::from_parameter(count), &count, |b, _| {
            b.iter(|| black_box(index.search(black_box(query), k).unwrap()));
        });
    }

    group.finish();
}

/// Benchmark: BQ search vs F32 search
///
/// Compares quantized Hamming distance search vs full F32 search.
fn bench_quantized_vs_f32(c: &mut Criterion) {
    let dims = 768;
    let k = 10;
    let seed = 42;
    let count = 5_000;

    // Generate vectors with clear binary patterns for BQ
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let vectors: Vec<Vec<f32>> = (0..count)
        .map(|_| {
            (0..dims)
                .map(|_| if rng.random_bool(0.5) { 1.0 } else { -1.0 })
                .collect()
        })
        .collect();

    let config = FlatIndexConfig::new(dims as u32);
    let mut index = FlatIndex::new(config);

    for v in &vectors {
        index.insert(v).unwrap();
    }

    let query = &vectors[0];

    let mut group = c.benchmark_group("flat_quantized_comparison");

    // F32 search
    group.bench_function("f32_5k_768d", |b| {
        b.iter(|| black_box(index.search(black_box(query), k).unwrap()));
    });

    // Enable quantization
    index.enable_quantization().unwrap();

    // BQ search
    group.bench_function("bq_5k_768d", |b| {
        b.iter(|| black_box(index.search_quantized(black_box(query), k).unwrap()));
    });

    group.finish();
}

/// Benchmark: Different distance metrics
fn bench_metrics(c: &mut Criterion) {
    let dims = 128;
    let k = 10;
    let seed = 42;
    let count = 5_000;

    let vectors = generate_vectors(count, dims, seed);
    let query = &vectors[0];

    let mut group = c.benchmark_group("flat_metrics");

    for metric in [
        DistanceMetric::Cosine,
        DistanceMetric::DotProduct,
        DistanceMetric::L2,
    ] {
        let config = FlatIndexConfig::new(dims as u32).with_metric(metric);
        let mut index = FlatIndex::new(config);

        for v in &vectors {
            index.insert(v).unwrap();
        }

        group.bench_function(format!("{metric:?}"), |b| {
            b.iter(|| black_box(index.search(black_box(query), k).unwrap()));
        });
    }

    group.finish();
}

/// Benchmark: Snapshot round-trip
fn bench_snapshot(c: &mut Criterion) {
    let dims = 128;
    let seed = 42;
    let count = 5_000;

    let vectors = generate_vectors(count, dims, seed);
    let config = FlatIndexConfig::new(dims as u32);
    let mut index = FlatIndex::new(config);

    for v in &vectors {
        index.insert(v).unwrap();
    }

    let mut group = c.benchmark_group("flat_snapshot");

    // Serialize
    group.bench_function("to_snapshot_5k", |b| {
        b.iter(|| black_box(index.to_snapshot().unwrap()));
    });

    // Deserialize
    let snapshot = index.to_snapshot().unwrap();
    group.bench_function("from_snapshot_5k", |b| {
        b.iter(|| black_box(FlatIndex::from_snapshot(black_box(&snapshot)).unwrap()));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_insert_latency,
    bench_search_latency,
    bench_search_768d,
    bench_quantized_vs_f32,
    bench_metrics,
    bench_snapshot
);
criterion_main!(benches);
