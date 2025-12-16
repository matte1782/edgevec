//! NEON SIMD Hamming Distance Benchmarks
//!
//! Benchmarks comparing NEON-optimized vs portable Hamming distance.
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
//! | Size | Portable | NEON | Speedup |
//! |:-----|:---------|:-----|:--------|
//! | 64B  | ~80ns    | ~20ns| ~4x     |
//! | 256B | ~300ns   | ~50ns| ~6x     |
//! | 1KB  | ~1.2µs   | ~150ns| ~8x    |
//! | 4KB  | ~4.8µs   | ~500ns| ~10x   |
//!
//! Target: NEON ≥2x faster than portable (actual expectation: 4-10x)
//!
//! ## Note on x86
//!
//! On x86 targets, only the portable benchmark runs. NEON benchmarks are
//! compiled out via `#[cfg(target_arch = "aarch64")]`.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::quantization::simd::portable::hamming_distance_slice as portable_hamming;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

#[cfg(target_arch = "aarch64")]
use edgevec::simd::neon::hamming_distance_slice as neon_hamming;

/// Generate random byte vectors for benchmarking.
fn generate_byte_vectors(size: usize, seed: u64) -> (Vec<u8>, Vec<u8>) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let a: Vec<u8> = (0..size).map(|_| rng.gen()).collect();
    let b: Vec<u8> = (0..size).map(|_| rng.gen()).collect();
    (a, b)
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

criterion_group!(
    benches,
    bench_neon_vs_portable,
    bench_neon_embedding_sizes,
    bench_neon_batch,
    bench_neon_tail_handling,
);
criterion_main!(benches);
