//! Benchmarks for binary quantization.
//!
//! Run with: `cargo bench --bench bench_quantization`

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::quantization::binary::{
    BinaryQuantizer, QuantizedVector, BINARY_QUANTIZATION_DIM, QUANTIZED_VECTOR_SIZE,
};
use std::hint::black_box;

/// Benchmark binary quantization (f32[768] -> u8[96])
fn bench_quantize(c: &mut Criterion) {
    let quantizer = BinaryQuantizer::new();
    let vector: Vec<f32> = (0..BINARY_QUANTIZATION_DIM)
        .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
        .collect();

    c.bench_function("binary_quantize_768d", |b| {
        b.iter(|| black_box(quantizer.quantize(black_box(&vector))))
    });
}

/// Benchmark Hamming distance computation
fn bench_hamming_distance(c: &mut Criterion) {
    let mut group = c.benchmark_group("hamming_distance");
    group.throughput(Throughput::Bytes(QUANTIZED_VECTOR_SIZE as u64 * 2));

    // Create two different quantized vectors
    let q1 = QuantizedVector::from_bytes([0xAA; QUANTIZED_VECTOR_SIZE]); // 10101010...
    let q2 = QuantizedVector::from_bytes([0x55; QUANTIZED_VECTOR_SIZE]); // 01010101...

    group.bench_function(BenchmarkId::new("96_bytes", "different"), |b| {
        b.iter(|| black_box(black_box(&q1).hamming_distance(black_box(&q2))))
    });

    // Same vectors (best case - no differences)
    let q_same = QuantizedVector::from_bytes([0xAA; QUANTIZED_VECTOR_SIZE]);
    group.bench_function(BenchmarkId::new("96_bytes", "identical"), |b| {
        b.iter(|| black_box(black_box(&q_same).hamming_distance(black_box(&q_same))))
    });

    // Opposite vectors (worst case - all differences)
    let q_zeros = QuantizedVector::from_bytes([0x00; QUANTIZED_VECTOR_SIZE]);
    let q_ones = QuantizedVector::from_bytes([0xFF; QUANTIZED_VECTOR_SIZE]);
    group.bench_function(BenchmarkId::new("96_bytes", "opposite"), |b| {
        b.iter(|| black_box(black_box(&q_zeros).hamming_distance(black_box(&q_ones))))
    });

    group.finish();
}

/// Benchmark similarity computation (includes Hamming + normalization)
fn bench_similarity(c: &mut Criterion) {
    let q1 = QuantizedVector::from_bytes([0xAA; QUANTIZED_VECTOR_SIZE]);
    let q2 = QuantizedVector::from_bytes([0x55; QUANTIZED_VECTOR_SIZE]);

    c.bench_function("binary_similarity_768d", |b| {
        b.iter(|| black_box(black_box(&q1).similarity(black_box(&q2))))
    });
}

/// Benchmark end-to-end: quantize + hamming distance
fn bench_e2e(c: &mut Criterion) {
    let quantizer = BinaryQuantizer::new();
    let vec1: Vec<f32> = (0..BINARY_QUANTIZATION_DIM)
        .map(|i| (i as f32 / BINARY_QUANTIZATION_DIM as f32) - 0.5)
        .collect();
    let vec2: Vec<f32> = (0..BINARY_QUANTIZATION_DIM)
        .map(|i| ((i + 384) as f32 / BINARY_QUANTIZATION_DIM as f32) - 0.5)
        .collect();

    c.bench_function("binary_quantize_and_compare", |b| {
        b.iter(|| {
            let q1 = quantizer.quantize(black_box(&vec1));
            let q2 = quantizer.quantize(black_box(&vec2));
            black_box(q1.hamming_distance(&q2))
        })
    });
}

criterion_group!(
    benches,
    bench_quantize,
    bench_hamming_distance,
    bench_similarity,
    bench_e2e
);
criterion_main!(benches);
