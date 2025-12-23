//! SIMD vs Scalar Comparison Benchmark (Week 30 Day 2)
//!
//! Run with: `cargo bench --bench simd_comparison`
//!
//! This benchmark validates the 2-3x speedup target for SIMD operations.
//!
//! # Target Metrics
//!
//! | Metric | Scalar (Baseline) | SIMD Target | Improvement |
//! |:-------|:------------------|:------------|:------------|
//! | Dot Product (768-dim) | ~500ns | <200ns | 2.5x |
//! | L2 Distance (768-dim) | ~600ns | <250ns | 2.4x |
//! | Search (10k, k=10) | ~5ms | ~2ms | 2.5x |
//! | Hamming Distance (1024-bit) | ~100ns | <40ns | 2.5x |

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::hnsw::{HnswConfig, HnswIndex, SearchContext};
use edgevec::metric::{DotProduct, L2Squared, Metric};
use edgevec::quantization::binary::QuantizedVector;
use edgevec::storage::VectorStorage;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

fn generate_random_vector(dim: usize, seed: u64) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..dim).map(|_| rng.gen_range(-1.0..1.0)).collect()
}

fn generate_vectors(count: usize, dim: usize, seed: u64) -> Vec<Vec<f32>> {
    (0..count)
        .map(|i| generate_random_vector(dim, seed + i as u64))
        .collect()
}

fn generate_quantized_vector(seed: u64) -> QuantizedVector {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut data = [0u8; 96];
    for byte in &mut data {
        *byte = rng.gen();
    }
    QuantizedVector::from_bytes(data)
}

// ============================================================================
// DOT PRODUCT BENCHMARK (Multiple Dimensions)
// ============================================================================

fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("dot_product");

    for dim in [128, 256, 384, 512, 768, 1024, 1536] {
        let a = generate_random_vector(dim, 42);
        let b = generate_random_vector(dim, 43);

        group.throughput(Throughput::Elements(dim as u64));

        group.bench_with_input(BenchmarkId::new("simd", dim), &dim, |bench, _| {
            bench.iter(|| DotProduct::distance(black_box(&a), black_box(&b)))
        });
    }
    group.finish();
}

// ============================================================================
// L2 DISTANCE BENCHMARK (Multiple Dimensions)
// ============================================================================

fn bench_l2_squared(c: &mut Criterion) {
    let mut group = c.benchmark_group("l2_squared");

    for dim in [128, 256, 384, 512, 768, 1024, 1536] {
        let a = generate_random_vector(dim, 42);
        let b = generate_random_vector(dim, 43);

        group.throughput(Throughput::Elements(dim as u64));

        group.bench_with_input(BenchmarkId::new("simd", dim), &dim, |bench, _| {
            bench.iter(|| L2Squared::distance(black_box(&a), black_box(&b)))
        });
    }
    group.finish();
}

// ============================================================================
// COSINE SIMILARITY BENCHMARK (Multiple Dimensions)
// ============================================================================

fn bench_cosine(c: &mut Criterion) {
    let mut group = c.benchmark_group("cosine_similarity");

    for dim in [128, 256, 384, 512, 768, 1024, 1536] {
        let a = generate_random_vector(dim, 42);
        let b = generate_random_vector(dim, 43);

        // Cosine = 1 - (dot / (|a| * |b|))
        // We'll just measure dot product since cosine uses it internally
        let a_norm: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
        let b_norm: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

        group.throughput(Throughput::Elements(dim as u64));

        group.bench_with_input(BenchmarkId::new("simd", dim), &dim, |bench, _| {
            bench.iter(|| {
                let dot = DotProduct::distance(black_box(&a), black_box(&b));
                // Note: DotProduct returns negative dot product for HNSW,
                // so cosine similarity = 1 + dot/(a_norm*b_norm)
                let cosine = 1.0 + dot / (a_norm * b_norm);
                black_box(cosine)
            })
        });
    }
    group.finish();
}

// ============================================================================
// HAMMING DISTANCE BENCHMARK (Binary Quantization)
// ============================================================================

fn bench_hamming(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_distance");

    // 768-bit = 96 bytes (standard QuantizedVector size)
    let q1 = generate_quantized_vector(42);
    let q2 = generate_quantized_vector(43);

    group.throughput(Throughput::Bytes(96 * 2)); // 2 × 96-byte inputs

    group.bench_function("768bit", |b| {
        b.iter(|| black_box(&q1).hamming_distance(black_box(&q2)))
    });

    group.finish();
}

// ============================================================================
// SEARCH BENCHMARK (Multiple Collection Sizes)
// ============================================================================

fn bench_search(c: &mut Criterion) {
    let mut group = c.benchmark_group("search");
    group.sample_size(50); // Fewer samples for slower benchmarks

    for count in [1_000, 10_000] {
        let dims = 768;
        let seed = 42u64;

        // Build index
        let vectors = generate_vectors(count, dims, seed);
        let config = HnswConfig::new(dims as u32);
        let mut storage = VectorStorage::new(&config, None);
        let mut index = HnswIndex::new(config, &storage).unwrap();

        for v in &vectors {
            index.insert(v, &mut storage).unwrap();
        }

        let query = generate_random_vector(dims, 999);

        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::new("k=10", count), &count, |bench, _| {
            let mut ctx = SearchContext::new();
            bench.iter(|| {
                index
                    .search_with_context(black_box(&query), 10, &storage, &mut ctx)
                    .unwrap()
            })
        });
    }
    group.finish();
}

// ============================================================================
// DISTANCE BATCH BENCHMARK (10k distance calculations)
// ============================================================================

fn bench_distance_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("distance_batch_10k");

    for dim in [128, 384, 768] {
        let query = generate_random_vector(dim, 42);
        let vectors = generate_vectors(10_000, dim, 100);

        group.throughput(Throughput::Elements(10_000));

        // L2 Squared
        group.bench_with_input(BenchmarkId::new("l2", dim), &dim, |bench, _| {
            bench.iter(|| {
                for v in &vectors {
                    black_box(L2Squared::distance(black_box(&query), black_box(v)));
                }
            })
        });

        // Dot Product
        group.bench_with_input(BenchmarkId::new("dot", dim), &dim, |bench, _| {
            bench.iter(|| {
                for v in &vectors {
                    black_box(DotProduct::distance(black_box(&query), black_box(v)));
                }
            })
        });
    }
    group.finish();
}

// ============================================================================
// HAMMING BATCH BENCHMARK (10k distance calculations)
// ============================================================================

fn bench_hamming_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_batch_10k");

    let query = generate_quantized_vector(42);
    let vectors: Vec<QuantizedVector> = (0..10_000)
        .map(|i| generate_quantized_vector(100 + i))
        .collect();

    group.throughput(Throughput::Elements(10_000));

    group.bench_function("768bit", |b| {
        b.iter(|| {
            for v in &vectors {
                black_box(black_box(&query).hamming_distance(black_box(v)));
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_dot_product,
    bench_l2_squared,
    bench_cosine,
    bench_hamming,
    bench_search,
    bench_distance_batch,
    bench_hamming_batch
);
criterion_main!(benches);
