use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use edgevec::quantization::ScalarQuantizer;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::hint::black_box;

/// Benchmark: Quantization Speed
///
/// Measure the time it takes to quantize vectors of different dimensions.
/// This is the "hot path" for insertion.
///
/// Target: < 50µs per 1536d vector (OpenAI embedding size).
fn bench_quantization_speed(c: &mut Criterion) {
    let seed = 42;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    // We care about standard embedding sizes
    let dimensions = [768, 1536];

    let mut group = c.benchmark_group("quantization_latency");

    for &dim in &dimensions {
        // Generate a random vector
        let vector: Vec<f32> = (0..dim).map(|_| rng.gen_range(-10.0..10.0)).collect();

        // Train a quantizer (once)
        let batch = vec![vector.as_slice()];
        let quantizer = ScalarQuantizer::train(&batch);

        group.throughput(Throughput::Elements(1));

        group.bench_with_input(BenchmarkId::from_parameter(dim), &dim, |b, _| {
            b.iter(|| {
                // Measure just the quantization step
                black_box(quantizer.quantize(black_box(&vector)))
            });
        });
    }

    group.finish();
}

/// Benchmark: Quantization Throughput (Batch)
///
/// Measure throughput in vectors/second for bulk operations.
fn bench_quantization_throughput(c: &mut Criterion) {
    let seed = 42;
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let dim = 1536;
    let batch_size = 1000;

    let vectors: Vec<Vec<f32>> = (0..batch_size)
        .map(|_| (0..dim).map(|_| rng.gen_range(-10.0..10.0)).collect())
        .collect();

    // Pre-train
    let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();
    let quantizer = ScalarQuantizer::train(&refs);

    let mut group = c.benchmark_group("quantization_throughput");
    group.throughput(Throughput::Elements(batch_size as u64));

    group.bench_function("quantize_1k_1536d", |b| {
        b.iter(|| {
            for v in &vectors {
                black_box(quantizer.quantize(black_box(v)));
            }
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_quantization_speed,
    bench_quantization_throughput
);
criterion_main!(benches);
