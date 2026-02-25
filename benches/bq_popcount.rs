//! Benchmarks for Binary Quantization SIMD Popcount.
//!
//! Run with: `cargo bench --bench bq_popcount`
//!
//! ## Performance Targets (Week 27)
//!
//! | Dimension | SIMD Target | Scalar Baseline | Expected Speedup |
//! |:----------|:------------|:----------------|:-----------------|
//! | 128D (16B) | <10ns | ~50ns | >5x |
//! | 384D (48B) | <20ns | ~100ns | >5x |
//! | 768D (96B) | <30ns | ~200ns | >6x |
//! | 1024D (128B) | <40ns | ~270ns | >6x |
//! | 1536D (192B) | <50ns | ~400ns | >8x |
//!
//! ## Methodology
//!
//! - Hardware: Runtime-detected SIMD (AVX2/popcnt/NEON/scalar)
//! - Measurement: Criterion with 100 sample iterations
//! - RNG: Deterministic seeded ChaCha8Rng for reproducibility

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::quantization::variable::BinaryVector;
use edgevec::simd::popcount::{scalar_popcount_xor, simd_popcount_xor};
use rand::{Rng, RngExt, SeedableRng};
use rand_chacha::ChaCha8Rng;

/// Generate a random f32 vector of given dimension.
fn generate_f32_vector(dims: usize, seed: u64) -> Vec<f32> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..dims).map(|_| rng.random_range(-1.0..1.0)).collect()
}

/// Generate a random byte vector of given length.
fn generate_byte_vector(bytes: usize, seed: u64) -> Vec<u8> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..bytes).map(|_| rng.random()).collect()
}

// ============================================================================
// SIMD vs SCALAR POPCOUNT BENCHMARKS
// ============================================================================

/// Benchmark SIMD popcount_xor for various byte lengths.
///
/// Tests the core SIMD popcount function directly.
fn bench_simd_popcount_xor(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_popcount_xor");

    // Common embedding dimensions → byte counts
    // 128D → 16B, 384D → 48B, 768D → 96B, 1024D → 128B, 1536D → 192B
    for bytes in [16, 48, 96, 128, 192] {
        let a = generate_byte_vector(bytes, 42);
        let b = generate_byte_vector(bytes, 43);

        group.throughput(Throughput::Bytes((bytes * 2) as u64));

        group.bench_with_input(BenchmarkId::new("simd", bytes), &bytes, |bench, _| {
            bench.iter(|| simd_popcount_xor(black_box(&a), black_box(&b)))
        });

        group.bench_with_input(BenchmarkId::new("scalar", bytes), &bytes, |bench, _| {
            bench.iter(|| scalar_popcount_xor(black_box(&a), black_box(&b)))
        });
    }

    group.finish();
}

/// Benchmark end-to-end Hamming distance via BinaryVector.
///
/// Measures the full workflow including dimension validation.
fn bench_binaryvector_hamming(c: &mut Criterion) {
    let mut group = c.benchmark_group("binaryvector_hamming");

    for dims in [128, 384, 768, 1024, 1536] {
        let v1 = generate_f32_vector(dims, 42);
        let v2 = generate_f32_vector(dims, 43);
        let bv1 = BinaryVector::quantize(&v1).unwrap();
        let bv2 = BinaryVector::quantize(&v2).unwrap();

        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(dims), &dims, |bench, _| {
            bench.iter(|| black_box(&bv1).hamming_distance(black_box(&bv2)))
        });
    }

    group.finish();
}

/// Benchmark quantization for various dimensions.
fn bench_binaryvector_quantize(c: &mut Criterion) {
    let mut group = c.benchmark_group("binaryvector_quantize");

    for dims in [128, 384, 768, 1024, 1536] {
        let v = generate_f32_vector(dims, 42);

        group.throughput(Throughput::Elements(dims as u64));

        group.bench_with_input(BenchmarkId::from_parameter(dims), &dims, |bench, _| {
            bench.iter(|| BinaryVector::quantize(black_box(&v)))
        });
    }

    group.finish();
}

/// Benchmark batch Hamming distance (realistic query scenario).
///
/// Measures performance when comparing one query against many vectors.
fn bench_batch_hamming(c: &mut Criterion) {
    let mut group = c.benchmark_group("batch_hamming");

    let batch_size = 1000;
    let dims = 768;

    // Generate batch of binary vectors
    let vectors: Vec<BinaryVector> = (0..batch_size)
        .map(|i| {
            let v = generate_f32_vector(dims, i as u64);
            BinaryVector::quantize(&v).unwrap()
        })
        .collect();
    let query = {
        let v = generate_f32_vector(dims, 9999);
        BinaryVector::quantize(&v).unwrap()
    };

    group.throughput(Throughput::Elements(batch_size as u64));

    group.bench_function("batch_1000x768d", |bench| {
        bench.iter(|| {
            let mut results = Vec::with_capacity(batch_size);
            for v in &vectors {
                results.push(black_box(&query).hamming_distance(black_box(v)));
            }
            results
        })
    });

    group.finish();
}

/// Benchmark similarity computation.
fn bench_binaryvector_similarity(c: &mut Criterion) {
    let mut group = c.benchmark_group("binaryvector_similarity");

    for dims in [128, 768, 1536] {
        let v1 = generate_f32_vector(dims, 42);
        let v2 = generate_f32_vector(dims, 43);
        let bv1 = BinaryVector::quantize(&v1).unwrap();
        let bv2 = BinaryVector::quantize(&v2).unwrap();

        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(dims), &dims, |bench, _| {
            bench.iter(|| black_box(&bv1).similarity(black_box(&bv2)))
        });
    }

    group.finish();
}

// ============================================================================
// SPEEDUP VERIFICATION
// ============================================================================

/// Compute speedup ratio for documentation.
///
/// This benchmark generates a summary table showing SIMD vs scalar speedup.
fn bench_speedup_summary(c: &mut Criterion) {
    let mut group = c.benchmark_group("speedup_summary");
    group.sample_size(200);

    // 768D case (most common embedding dimension)
    let a = generate_byte_vector(96, 42);
    let b = generate_byte_vector(96, 43);

    group.bench_function("768d_simd", |bench| {
        bench.iter(|| simd_popcount_xor(black_box(&a), black_box(&b)))
    });

    group.bench_function("768d_scalar", |bench| {
        bench.iter(|| scalar_popcount_xor(black_box(&a), black_box(&b)))
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_simd_popcount_xor,
    bench_binaryvector_hamming,
    bench_binaryvector_quantize,
    bench_batch_hamming,
    bench_binaryvector_similarity,
    bench_speedup_summary,
);
criterion_main!(benches);
