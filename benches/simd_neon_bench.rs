//! NEON SIMD Benchmarks
//!
//! Benchmarks comparing NEON-optimized vs portable implementations for:
//! - Hamming distance (binary vectors)
//! - Dot product (f32 vectors)
//! - Euclidean distance (f32 vectors)
//!
//! ## Running on ARM64
//!
//! **Native ARM64:**
//! ```bash
//! cargo bench --bench simd_neon_bench
//! ```
//!
//! **Cross-compile for ARM64 (from x86):**
//! ```bash
//! cross bench --target aarch64-unknown-linux-gnu --bench simd_neon_bench
//! ```
//!
//! ## Expected Results (ARM64)
//!
//! ### Hamming Distance
//!
//! | Size | Portable | NEON | Speedup |
//! |:-----|:---------|:-----|:--------|
//! | 64B  | ~80ns    | ~20ns| ~4x     |
//! | 256B | ~300ns   | ~50ns| ~6x     |
//! | 1KB  | ~1.2µs   | ~150ns| ~8x    |
//! | 4KB  | ~4.8µs   | ~500ns| ~10x   |
//!
//! ### Similarity Functions (Dot Product, Euclidean)
//!
//! | Dims | Portable | NEON | Speedup |
//! |:-----|:---------|:-----|:--------|
//! | 128  | ~200ns   | ~80ns| ~2.5x   |
//! | 768  | ~1.2µs   | ~300ns| ~4x    |
//! | 1536 | ~2.4µs   | ~600ns| ~4x    |
//!
//! Target: NEON ≥2x faster than portable
//!
//! ## Note on x86
//!
//! On x86 targets, only the portable benchmark runs. NEON benchmarks are
//! compiled out via `#[cfg(target_arch = "aarch64")]`.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::quantization::simd::portable::hamming_distance_slice as portable_hamming;
use rand::{Rng, RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[cfg(target_arch = "aarch64")]
use edgevec::simd::neon::{
    dot_product as neon_dot_product, dot_product_portable,
    euclidean_distance as neon_euclidean, euclidean_distance_portable,
    hamming_distance_slice as neon_hamming,
};

/// Generate random byte vectors for benchmarking.
fn generate_byte_vectors(size: usize, seed: u64) -> (Vec<u8>, Vec<u8>) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let a: Vec<u8> = (0..size).map(|_| rng.random()).collect();
    let b: Vec<u8> = (0..size).map(|_| rng.random()).collect();
    (a, b)
}

/// Generate random f32 vectors for benchmarking.
fn generate_f32_vectors(size: usize, seed: u64) -> (Vec<f32>, Vec<f32>) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let a: Vec<f32> = (0..size).map(|_| rng.random_range(-1.0..1.0)).collect();
    let b: Vec<f32> = (0..size).map(|_| rng.random_range(-1.0..1.0)).collect();
    (a, b)
}

/// Portable dot product for non-ARM benchmarks
#[cfg(not(target_arch = "aarch64"))]
fn portable_dot_product(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b.iter()).map(|(x, y)| x * y).sum()
}

/// Portable euclidean distance for non-ARM benchmarks
#[cfg(not(target_arch = "aarch64"))]
fn portable_euclidean(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b.iter())
        .map(|(x, y)| {
            let diff = x - y;
            diff * diff
        })
        .sum::<f32>()
        .sqrt()
}

/// Benchmark NEON vs Portable Hamming distance.
///
/// Tests various input sizes to understand performance characteristics.
fn bench_neon_vs_portable(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_neon_vs_portable");

    // Test sizes: 64, 256, 1024, 4096 bytes
    // These cover:
    // - 64B: Small vectors (4 NEON chunks)
    // - 256B: Medium vectors (16 NEON chunks)
    // - 1024B: Large vectors (64 NEON chunks)
    // - 4096B: Very large vectors (256 NEON chunks)
    let sizes = [64, 256, 1024, 4096];

    for size in sizes {
        let (a, b) = generate_byte_vectors(size, 42 + size as u64);

        group.throughput(Throughput::Bytes((size * 2) as u64)); // 2 input vectors

        // Portable baseline (runs on all platforms)
        group.bench_with_input(BenchmarkId::new("portable", size), &size, |bench, _| {
            bench.iter(|| portable_hamming(black_box(&a), black_box(&b)));
        });

        // NEON optimized (ARM64 only)
        #[cfg(target_arch = "aarch64")]
        group.bench_with_input(BenchmarkId::new("neon", size), &size, |bench, _| {
            bench.iter(|| neon_hamming(black_box(&a), black_box(&b)));
        });
    }

    group.finish();
}

/// Benchmark NEON Hamming with realistic embedding sizes.
///
/// Common embedding dimensions that map to binary vectors:
/// - 768 bits = 96 bytes (standard BERT)
/// - 1536 bits = 192 bytes (OpenAI ada-002)
/// - 3072 bits = 384 bytes (large models)
fn bench_neon_embedding_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_embedding_sizes");

    let sizes = [96, 192, 384]; // Common binary embedding sizes in bytes

    for size in sizes {
        let (a, b) = generate_byte_vectors(size, 100 + size as u64);

        group.throughput(Throughput::Bytes((size * 2) as u64));

        group.bench_with_input(BenchmarkId::new("portable", size), &size, |bench, _| {
            bench.iter(|| portable_hamming(black_box(&a), black_box(&b)));
        });

        #[cfg(target_arch = "aarch64")]
        group.bench_with_input(BenchmarkId::new("neon", size), &size, |bench, _| {
            bench.iter(|| neon_hamming(black_box(&a), black_box(&b)));
        });
    }

    group.finish();
}

/// Benchmark batch Hamming distance operations.
///
/// Measures throughput when computing many distances (realistic search use case).
fn bench_neon_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_batch");

    let batch_size = 1000;
    let vector_size = 96; // Standard 768-bit binary embedding

    // Generate batch of vectors
    let vectors: Vec<Vec<u8>> = (0..batch_size)
        .map(|i| {
            let (v, _) = generate_byte_vectors(vector_size, i as u64);
            v
        })
        .collect();
    let (query, _) = generate_byte_vectors(vector_size, 9999);

    group.throughput(Throughput::Elements(batch_size as u64));

    // Portable batch
    group.bench_function("portable_batch_1000", |bench| {
        bench.iter(|| {
            for v in &vectors {
                black_box(portable_hamming(black_box(&query), black_box(v)));
            }
        });
    });

    // NEON batch (ARM64 only)
    #[cfg(target_arch = "aarch64")]
    group.bench_function("neon_batch_1000", |bench| {
        bench.iter(|| {
            for v in &vectors {
                black_box(neon_hamming(black_box(&query), black_box(v)));
            }
        });
    });

    group.finish();
}

/// Benchmark edge cases: tail handling.
///
/// Tests sizes that exercise the tail path (len % 16 != 0).
fn bench_neon_tail_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_tail_handling");

    // Sizes that test different tail lengths
    let sizes = [
        15,  // 0 chunks + 15 tail
        17,  // 1 chunk + 1 tail
        31,  // 1 chunk + 15 tail
        33,  // 2 chunks + 1 tail
        100, // 6 chunks + 4 tail
    ];

    for size in sizes {
        let (a, b) = generate_byte_vectors(size, 200 + size as u64);

        group.bench_with_input(BenchmarkId::new("portable", size), &size, |bench, _| {
            bench.iter(|| portable_hamming(black_box(&a), black_box(&b)));
        });

        #[cfg(target_arch = "aarch64")]
        group.bench_with_input(BenchmarkId::new("neon", size), &size, |bench, _| {
            bench.iter(|| neon_hamming(black_box(&a), black_box(&b)));
        });
    }

    group.finish();
}

// =============================================================================
// Similarity Benchmarks (Dot Product & Euclidean Distance)
// =============================================================================

/// Benchmark NEON vs Portable dot product.
///
/// Tests at common embedding dimensions: 128, 768, 1536
fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("dot_product_neon_vs_portable");

    // Common embedding dimensions
    let dims = [128, 768, 1536];

    for dim in dims {
        let (a, b) = generate_f32_vectors(dim, 300 + dim as u64);

        group.throughput(Throughput::Elements(dim as u64));

        // Portable baseline
        #[cfg(not(target_arch = "aarch64"))]
        group.bench_with_input(BenchmarkId::new("portable", dim), &dim, |bench, _| {
            bench.iter(|| portable_dot_product(black_box(&a), black_box(&b)));
        });

        #[cfg(target_arch = "aarch64")]
        {
            group.bench_with_input(BenchmarkId::new("portable", dim), &dim, |bench, _| {
                bench.iter(|| dot_product_portable(black_box(&a), black_box(&b)));
            });

            group.bench_with_input(BenchmarkId::new("neon", dim), &dim, |bench, _| {
                bench.iter(|| neon_dot_product(black_box(&a), black_box(&b)));
            });
        }
    }

    group.finish();
}

/// Benchmark NEON vs Portable euclidean distance.
///
/// Tests at common embedding dimensions: 128, 768, 1536
fn bench_euclidean_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("euclidean_neon_vs_portable");

    let dims = [128, 768, 1536];

    for dim in dims {
        let (a, b) = generate_f32_vectors(dim, 400 + dim as u64);

        group.throughput(Throughput::Elements(dim as u64));

        #[cfg(not(target_arch = "aarch64"))]
        group.bench_with_input(BenchmarkId::new("portable", dim), &dim, |bench, _| {
            bench.iter(|| portable_euclidean(black_box(&a), black_box(&b)));
        });

        #[cfg(target_arch = "aarch64")]
        {
            group.bench_with_input(BenchmarkId::new("portable", dim), &dim, |bench, _| {
                bench.iter(|| euclidean_distance_portable(black_box(&a), black_box(&b)));
            });

            group.bench_with_input(BenchmarkId::new("neon", dim), &dim, |bench, _| {
                bench.iter(|| neon_euclidean(black_box(&a), black_box(&b)));
            });
        }
    }

    group.finish();
}

/// Benchmark batch similarity operations.
///
/// Simulates realistic search scenario: 1 query vs 1000 vectors.
fn bench_similarity_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("similarity_batch");

    let batch_size = 1000;
    let dim = 768; // OpenAI embedding dimension

    let vectors: Vec<Vec<f32>> = (0..batch_size)
        .map(|i| {
            let (v, _) = generate_f32_vectors(dim, i as u64);
            v
        })
        .collect();
    let (query, _) = generate_f32_vectors(dim, 9999);

    group.throughput(Throughput::Elements(batch_size as u64));

    // Dot product batch
    #[cfg(not(target_arch = "aarch64"))]
    group.bench_function("dot_portable_batch_1000", |bench| {
        bench.iter(|| {
            for v in &vectors {
                black_box(portable_dot_product(black_box(&query), black_box(v)));
            }
        });
    });

    #[cfg(target_arch = "aarch64")]
    {
        group.bench_function("dot_portable_batch_1000", |bench| {
            bench.iter(|| {
                for v in &vectors {
                    black_box(dot_product_portable(black_box(&query), black_box(v)));
                }
            });
        });

        group.bench_function("dot_neon_batch_1000", |bench| {
            bench.iter(|| {
                for v in &vectors {
                    black_box(neon_dot_product(black_box(&query), black_box(v)));
                }
            });
        });

        group.bench_function("euclidean_portable_batch_1000", |bench| {
            bench.iter(|| {
                for v in &vectors {
                    black_box(euclidean_distance_portable(black_box(&query), black_box(v)));
                }
            });
        });

        group.bench_function("euclidean_neon_batch_1000", |bench| {
            bench.iter(|| {
                for v in &vectors {
                    black_box(neon_euclidean(black_box(&query), black_box(v)));
                }
            });
        });
    }

    #[cfg(not(target_arch = "aarch64"))]
    group.bench_function("euclidean_portable_batch_1000", |bench| {
        bench.iter(|| {
            for v in &vectors {
                black_box(portable_euclidean(black_box(&query), black_box(v)));
            }
        });
    });

    group.finish();
}

/// Benchmark tail handling for similarity functions.
///
/// Tests sizes that exercise scalar tail path (len % 4 != 0).
fn bench_similarity_tail_handling(c: &mut Criterion) {
    let mut group = c.benchmark_group("similarity_tail_handling");

    // Sizes that exercise different tail lengths for f32 (4 floats per NEON vector)
    let sizes = [
        1,   // 0 chunks + 1 tail
        3,   // 0 chunks + 3 tail
        5,   // 1 chunk + 1 tail
        7,   // 1 chunk + 3 tail
        100, // 25 chunks + 0 tail
        103, // 25 chunks + 3 tail
    ];

    for size in sizes {
        let (a, b) = generate_f32_vectors(size, 500 + size as u64);

        // Dot product
        #[cfg(not(target_arch = "aarch64"))]
        group.bench_with_input(BenchmarkId::new("dot_portable", size), &size, |bench, _| {
            bench.iter(|| portable_dot_product(black_box(&a), black_box(&b)));
        });

        #[cfg(target_arch = "aarch64")]
        {
            group.bench_with_input(BenchmarkId::new("dot_portable", size), &size, |bench, _| {
                bench.iter(|| dot_product_portable(black_box(&a), black_box(&b)));
            });

            group.bench_with_input(BenchmarkId::new("dot_neon", size), &size, |bench, _| {
                bench.iter(|| neon_dot_product(black_box(&a), black_box(&b)));
            });
        }
    }

    group.finish();
}

criterion_group!(
    benches,
    // Hamming benchmarks
    bench_neon_vs_portable,
    bench_neon_embedding_sizes,
    bench_neon_batch,
    bench_neon_tail_handling,
    // Similarity benchmarks
    bench_dot_product,
    bench_euclidean_distance,
    bench_similarity_batch,
    bench_similarity_tail_handling,
);
criterion_main!(benches);
