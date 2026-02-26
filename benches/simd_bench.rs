//! Benchmarks for SIMD vs Scalar distance metrics.
//!
//! Run with: `cargo bench --bench simd_bench`
//!
//! ## SIMD Hamming Distance Targets (Week 8 Day 37)
//!
//! | Metric | Target | Hard Limit | Evidence |
//! |:-------|:-------|:-----------|:---------|
//! | AVX2 Cycles | <50 | <75 | rdtsc measurement |
//! | Speedup | >5x | >3x | criterion comparison |
//! | Throughput | >1B ops/sec | >500M ops/sec | criterion |
//! | Latency P99 | <100ns | <200ns | criterion |
//!
//! See: `docs/benchmarks/SIMD_TARGETS.md` for full specification.

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::hint::black_box;
use edgevec::metric::scalar;
#[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
use edgevec::metric::simd::x86;
use edgevec::metric::{DotProduct, L2Squared, Metric};
use edgevec::quantization::binary::QuantizedVector;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn generate_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0..1.0)).collect())
        .collect()
}

fn generate_u8_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<u8>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(0..255)).collect())
        .collect()
}

// ============================================================================
// SIMD HAMMING DISTANCE BENCHMARKS (Week 8 Day 37)
// ============================================================================

/// Generate random QuantizedVector for benchmarking.
fn generate_quantized_vector(seed: u64) -> QuantizedVector {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let mut data = [0u8; 96];
    for byte in &mut data {
        *byte = rng.gen();
    }
    QuantizedVector::from_bytes(data)
}

/// PRIMARY TARGET: <50 cycles per Hamming distance call.
///
/// This benchmark measures the core performance of SIMD Hamming distance.
/// Target: <50 cycles (Hard limit: <75 cycles)
fn bench_simd_hamming_cycles(c: &mut Criterion) {
    let q1 = QuantizedVector::from_bytes([0xAAu8; 96]);
    let q2 = QuantizedVector::from_bytes([0x55u8; 96]);

    c.bench_function("simd_hamming_96bytes", |b| {
        b.iter(|| black_box(&q1).hamming_distance(black_box(&q2)))
    });
}

/// TARGET: >5x speedup over portable implementation.
///
/// Compares SIMD vs portable Hamming distance performance.
/// Target: >5x speedup (Hard limit: >3x)
fn bench_simd_vs_portable(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_hamming_comparison");
    group.throughput(Throughput::Bytes(96 * 2)); // 2 × 96-byte inputs

    let q1 = QuantizedVector::from_bytes([0xAAu8; 96]);
    let q2 = QuantizedVector::from_bytes([0x55u8; 96]);

    // SIMD path (uses runtime dispatch internally)
    group.bench_function("simd_dispatch", |b| {
        b.iter(|| black_box(&q1).hamming_distance(black_box(&q2)))
    });

    // Portable baseline (direct byte-by-byte)
    group.bench_function("portable_baseline", |b| {
        b.iter(|| {
            // Manual portable implementation for comparison
            let a = black_box(q1.data());
            let b = black_box(q2.data());
            let mut distance = 0u32;
            for i in 0..96 {
                distance += (a[i] ^ b[i]).count_ones();
            }
            black_box(distance)
        })
    });

    group.finish();
}

/// TARGET: >1 billion operations per second throughput.
///
/// Measures sustained throughput for Hamming distance operations.
/// Target: >1B ops/sec (Hard limit: >500M ops/sec)
fn bench_simd_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_hamming_throughput");
    group.throughput(Throughput::Elements(1)); // 1 Hamming distance = 1 element

    let q1 = QuantizedVector::from_bytes([0xAAu8; 96]);
    let q2 = QuantizedVector::from_bytes([0x55u8; 96]);

    group.bench_function("hamming_ops_per_sec", |b| {
        b.iter(|| black_box(&q1).hamming_distance(black_box(&q2)))
    });

    group.finish();
}

/// Diverse input patterns for realistic benchmarking.
///
/// Tests various bit patterns to ensure consistent performance.
fn bench_simd_diverse_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_hamming_patterns");

    let zeros = QuantizedVector::from_bytes([0x00u8; 96]);
    let ones = QuantizedVector::from_bytes([0xFFu8; 96]);
    let alt_aa = QuantizedVector::from_bytes([0xAAu8; 96]);
    let alt_55 = QuantizedVector::from_bytes([0x55u8; 96]);
    let random1 = generate_quantized_vector(42);
    let random2 = generate_quantized_vector(43);

    // Minimum distance case (identical vectors)
    group.bench_function("pattern_zeros_identical", |b| {
        b.iter(|| black_box(&zeros).hamming_distance(black_box(&zeros)))
    });

    // Maximum distance case (all bits differ)
    group.bench_function("pattern_ones_vs_zeros", |b| {
        b.iter(|| black_box(&ones).hamming_distance(black_box(&zeros)))
    });

    // Alternating patterns (all bits differ)
    group.bench_function("pattern_alternating", |b| {
        b.iter(|| black_box(&alt_aa).hamming_distance(black_box(&alt_55)))
    });

    // Random realistic data
    group.bench_function("pattern_random", |b| {
        b.iter(|| black_box(&random1).hamming_distance(black_box(&random2)))
    });

    group.finish();
}

/// Batch Hamming distance benchmark.
///
/// Measures performance when computing many distances (realistic use case).
fn bench_simd_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("simd_hamming_batch");

    // Generate batch of quantized vectors
    let batch_size = 1000;
    let vectors: Vec<QuantizedVector> = (0..batch_size)
        .map(|i| generate_quantized_vector(i as u64))
        .collect();
    let query = generate_quantized_vector(9999);

    group.throughput(Throughput::Elements(batch_size as u64));

    group.bench_function("batch_1000_vectors", |b| {
        b.iter(|| {
            for v in &vectors {
                black_box(black_box(&query).hamming_distance(black_box(v)));
            }
        })
    });

    group.finish();
}

fn bench_l2_squared(c: &mut Criterion) {
    let mut group = c.benchmark_group("l2_squared");
    let count = 10_000;

    for dims in [128, 768, 1536] {
        let vectors = generate_vectors(count + 1, dims, 42);
        let query = &vectors[0];
        let targets = &vectors[1..];

        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(dims), &dims, |b, _| {
            b.iter(|| {
                for target in targets {
                    black_box(L2Squared::distance(black_box(query), black_box(target)));
                }
            });
        });
    }
    group.finish();
}

fn bench_l2_squared_u8(c: &mut Criterion) {
    let mut group = c.benchmark_group("l2_squared_u8");
    let count = 10_000;

    for dims in [128, 768, 1536] {
        let vectors = generate_u8_vectors(count + 1, dims, 42);
        let query = &vectors[0];
        let targets = &vectors[1..];

        group.throughput(Throughput::Elements(count as u64));

        // Scalar Baseline
        group.bench_with_input(BenchmarkId::new("scalar", dims), &dims, |b, _| {
            b.iter(|| {
                for target in targets {
                    black_box(scalar::l2_squared_u8(black_box(query), black_box(target)));
                }
            });
        });

        // AVX2 (if available)
        #[cfg(all(target_arch = "x86_64", target_feature = "avx2"))]
        {
            group.bench_with_input(BenchmarkId::new("avx2", dims), &dims, |b, _| {
                b.iter(|| {
                    for target in targets {
                        unsafe {
                            black_box(x86::l2_squared_u8(black_box(query), black_box(target)));
                        }
                    }
                });
            });
        }
    }
    group.finish();
}

fn bench_dot_product(c: &mut Criterion) {
    let mut group = c.benchmark_group("dot_product");
    let count = 10_000;

    for dims in [128, 768] {
        let vectors = generate_vectors(count + 1, dims, 42);
        let query = &vectors[0];
        let targets = &vectors[1..];

        group.throughput(Throughput::Elements(count as u64));

        group.bench_with_input(BenchmarkId::from_parameter(dims), &dims, |b, _| {
            b.iter(|| {
                for target in targets {
                    black_box(DotProduct::distance(black_box(query), black_box(target)));
                }
            });
        });
    }
    group.finish();
}

// ============================================================================
// RDTSC CYCLE MEASUREMENT (Week 8 Day 37)
// ============================================================================

/// Measure CPU cycles using rdtsc.
///
/// Protocol:
/// - Warmup: 1,000 iterations
/// - Measurement: 10,000 iterations
/// - Returns: Average cycles per operation
#[cfg(target_arch = "x86_64")]
pub fn measure_cycles<F>(f: F, iterations: u64) -> u64
where
    F: Fn() -> u32,
{
    use std::arch::x86_64::_rdtsc;

    // Warmup: 1,000 iterations to ensure code in L1 cache
    for _ in 0..1000 {
        std::hint::black_box(f());
    }

    // Measurement: iterations (default 10,000) for statistical significance
    let start = unsafe { _rdtsc() };
    for _ in 0..iterations {
        std::hint::black_box(f());
    }
    let end = unsafe { _rdtsc() };

    (end - start) / iterations
}

/// Test that SIMD Hamming distance meets <50 cycle target.
///
/// This test will FAIL until SIMD implementation exists and meets target.
#[test]
#[cfg(target_arch = "x86_64")]
fn test_simd_cycle_target() {
    let q1 = QuantizedVector::from_bytes([0xAAu8; 96]);
    let q2 = QuantizedVector::from_bytes([0x55u8; 96]);

    let cycles = measure_cycles(|| q1.hamming_distance(&q2), 10_000);

    println!("=================================================");
    println!("SIMD Hamming Distance Cycle Measurement");
    println!("=================================================");
    println!("Measured cycles: {}", cycles);
    println!("Target: <50 cycles");
    println!("Hard limit: <75 cycles");
    println!("=================================================");

    // Hard limit check (MUST pass)
    assert!(
        cycles < 75,
        "FAIL: {} cycles exceeds hard limit of 75 cycles",
        cycles
    );

    // Target check (SHOULD pass for full approval)
    if cycles >= 50 {
        println!("WARNING: {} cycles exceeds target of 50 cycles", cycles);
        println!("Implementation meets hard limit but not target.");
    } else {
        println!("SUCCESS: {} cycles meets <50 cycle target", cycles);
    }
}

criterion_group!(
    benches,
    // SIMD Hamming benchmarks (Week 8 Day 37)
    bench_simd_hamming_cycles,
    bench_simd_vs_portable,
    bench_simd_throughput,
    bench_simd_diverse_patterns,
    bench_simd_batch,
    // Original L2/dot product benchmarks
    bench_l2_squared,
    bench_l2_squared_u8,
    bench_dot_product
);
criterion_main!(benches);
