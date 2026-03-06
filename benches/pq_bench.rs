//! PQ Benchmark Suite — W46 Days 4-5
//!
//! Benchmarks for Product Quantization: encoding speed (B1), ADC search
//! latency (B2), recall@10 (B3/B3b), codebook training (B5/B7), and
//! memory footprint (B6).
//!
//! Run: `cargo bench --bench pq_bench`
//! Run single: `cargo bench --bench pq_bench -- pq_encoding`

use criterion::{criterion_group, criterion_main, Criterion, Throughput};
use edgevec::quantization::product::{PqCode, PqCodebook};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::hint::black_box;

// =========================================================================
// Dataset generation (canonical, matches PQ_BENCHMARK_PLAN Section 2.2)
// =========================================================================

const DIMS: usize = 768;
const SEED_BASE: u64 = 42;
const SEED_QUERY: u64 = 137;

fn generate_dataset(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(-1.0f32..1.0f32)).collect())
        .collect()
}

// =========================================================================
// B1: PQ Encoding Speed (50K vectors, 768D, M=8, Ksub=256)
// =========================================================================

fn bench_pq_encoding(c: &mut Criterion) {
    let count = 50_000;
    let vectors = generate_dataset(count, DIMS, SEED_BASE);
    let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();

    // Train codebook OUTSIDE the benchmark loop
    let codebook = PqCodebook::train(&refs, 8, 256, 15).expect("training should succeed");

    let mut group = c.benchmark_group("pq_encoding");
    group.throughput(Throughput::Elements(count as u64));
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(3));

    group.bench_function("encode_50k_768d", |b| {
        b.iter(|| {
            let codes: Vec<PqCode> = vectors
                .iter()
                .map(|v| codebook.encode(black_box(v)).expect("encode"))
                .collect();
            black_box(codes)
        });
    });

    group.finish();
}

// =========================================================================
// B2: ADC Search Latency (100K codes, M=8, per-candidate)
// =========================================================================

fn bench_pq_adc_search(c: &mut Criterion) {
    let count = 100_000;
    let vectors = generate_dataset(count, DIMS, SEED_BASE);
    let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();
    let queries = generate_dataset(10, DIMS, SEED_QUERY);

    let codebook = PqCodebook::train(&refs, 8, 256, 15).expect("training should succeed");
    let codes: Vec<PqCode> = vectors
        .iter()
        .map(|v| codebook.encode(v).expect("encode"))
        .collect();

    let num_queries = queries.len() as u64;
    let mut group = c.benchmark_group("pq_adc_search");
    group.throughput(Throughput::Elements(count as u64 * num_queries));
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(3));

    group.bench_function("adc_100k_768d", |b| {
        b.iter(|| {
            for query in &queries {
                let dt = codebook
                    .compute_distance_table(black_box(query))
                    .expect("distance table");
                let results = dt.scan_topk(black_box(&codes), 10);
                black_box(results);
            }
        });
    });

    group.finish();
}

// =========================================================================
// B5: Codebook Training Time (50K, M=8, Ksub=256, 15 iters)
// =========================================================================

fn bench_pq_training_50k(c: &mut Criterion) {
    let count = 50_000;
    let vectors = generate_dataset(count, DIMS, SEED_BASE);
    let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();

    let mut group = c.benchmark_group("pq_training");
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(3));

    group.bench_function("train_50k_768d", |b| {
        b.iter(|| {
            let cb = PqCodebook::train(black_box(&refs), 8, 256, 15).expect("train");
            black_box(cb)
        });
    });

    group.finish();
}

// =========================================================================
// B7: Codebook Training Time (100K — G4 gate)
// =========================================================================

fn bench_pq_training_100k(c: &mut Criterion) {
    let count = 100_000;
    let vectors = generate_dataset(count, DIMS, SEED_BASE);
    let refs: Vec<&[f32]> = vectors.iter().map(|v| v.as_slice()).collect();

    let mut group = c.benchmark_group("pq_training_100k");
    group.sample_size(10);
    group.warm_up_time(std::time::Duration::from_secs(5));
    group.measurement_time(std::time::Duration::from_secs(600));

    group.bench_function("train_100k_768d", |b| {
        b.iter(|| {
            let cb = PqCodebook::train(black_box(&refs), 8, 256, 15).expect("train");
            black_box(cb)
        });
    });

    group.finish();
}

// =========================================================================
// B3: Recall@10 (not a Criterion bench — standalone measurement)
// =========================================================================

/// Compute exact k-nearest neighbors by brute-force L2 distance.
#[allow(dead_code)]
fn exact_knn(base: &[Vec<f32>], query: &[f32], k: usize) -> Vec<usize> {
    let mut dists: Vec<(usize, f32)> = base
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let d: f32 = v
                .iter()
                .zip(query.iter())
                .map(|(&a, &b)| {
                    let diff = a - b;
                    diff * diff
                })
                .sum();
            (i, d)
        })
        .collect();
    dists.sort_by(|a, b| a.1.total_cmp(&b.1));
    dists.iter().take(k).map(|&(idx, _)| idx).collect()
}

/// Measure recall@10 for PQ at a given dataset size.
#[allow(dead_code)]
fn measure_recall(
    base_count: usize,
    query_count: usize,
    m: usize,
    ksub: usize,
    iters: usize,
) -> f64 {
    let base = generate_dataset(base_count, DIMS, SEED_BASE);
    let queries = generate_dataset(query_count, DIMS, SEED_QUERY);
    let refs: Vec<&[f32]> = base.iter().map(|v| v.as_slice()).collect();

    let codebook = PqCodebook::train(&refs, m, ksub, iters).expect("train");
    let codes: Vec<PqCode> = base
        .iter()
        .map(|v| codebook.encode(v).expect("encode"))
        .collect();

    let mut total_recall = 0.0f64;

    for query in &queries {
        let true_topk: std::collections::HashSet<usize> =
            exact_knn(&base, query, 10).into_iter().collect();

        let dt = codebook
            .compute_distance_table(query)
            .expect("distance table");
        let pq_results = dt.scan_topk(&codes, 10);
        let pq_topk: std::collections::HashSet<usize> =
            pq_results.iter().map(|r| r.index).collect();

        let overlap = pq_topk.intersection(&true_topk).count();
        total_recall += overlap as f64 / 10.0;
    }

    total_recall / query_count as f64
}

criterion_group!(
    benches,
    bench_pq_encoding,
    bench_pq_adc_search,
    bench_pq_training_50k,
    bench_pq_training_100k,
);
criterion_main!(benches);
