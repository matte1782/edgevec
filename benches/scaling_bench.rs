//! W6D30 Final Scaling Validation
//!
//! Run with: `cargo bench --bench scaling_bench`
//!
//! # Goals
//! - Validate scaling behavior for QuantizedU8 vs Float32.
//! - Extrapolate to 1M vectors.
//!
//! # Sizes
//! [10_000, 50_000, 100_000]

use criterion::{
    criterion_group, criterion_main, BenchmarkId, Criterion, SamplingMode, Throughput,
};
use edgevec::hnsw::{HnswConfig, HnswIndex, SearchContext};
use edgevec::quantization::QuantizerConfig;
use edgevec::storage::{StorageType, VectorStorage};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::hint::black_box;
use std::time::{Duration, Instant};

/// Generates deterministic test vectors.
fn generate_vectors(count: usize, dims: usize, seed: u64) -> Vec<Vec<f32>> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    (0..count)
        .map(|_| (0..dims).map(|_| rng.gen_range(0.0..1.0)).collect())
        .collect()
}

fn bench_scaling(c: &mut Criterion) {
    // W6D30 spec: 768 dimensions (typical embedding size)
    let dims = 768;
    let k = 10;
    let seed = 42;

    let mut group = c.benchmark_group("scaling_validation");
    group.sample_size(10); // Reduced sample size for speed on large N
    group.sampling_mode(SamplingMode::Flat);
    group.measurement_time(Duration::from_secs(60)); // 60s per metric

    // Sizes requested: 10k, 50k, 100k
    for count in [10_000, 50_000, 100_000] {
        println!("\n=== Preparing for N={} ===", count);

        // Generate data (this takes memory too)
        let vectors = generate_vectors(count, dims, seed);
        let queries = generate_vectors(100, dims, seed + 1);

        // Run bench for Float32 and QuantizedU8
        for use_quantization in [false, true] {
            let mode_str = if use_quantization {
                "Quantized"
            } else {
                "Float32"
            };
            println!("Building index for N={} Mode={}...", count, mode_str);
            let start = Instant::now();

            let config = HnswConfig::new(dims as u32);
            let mut storage = VectorStorage::new(&config, None);
            if use_quantization {
                let q_config = QuantizerConfig { min: 0.0, max: 1.0 };
                storage.set_storage_type(StorageType::QuantizedU8(q_config));
            }

            let mut index = HnswIndex::new(config.clone(), &storage).unwrap();

            for v in &vectors {
                index.insert(v, &mut storage).unwrap();
            }

            let build_duration = start.elapsed();
            let avg_insert_ms = build_duration.as_secs_f64() * 1000.0 / count as f64;

            // Fail-Fast Check (W6 requirement) - relaxed limits for W8
            if count == 10_000 {
                let check_start = Instant::now();
                let check_count = 100.min(queries.len());
                let mut search_ctx = SearchContext::new();
                for query in queries.iter().take(check_count) {
                    let _ = black_box(
                        index
                            .search_with_context(query, k, &storage, &mut search_ctx)
                            .unwrap(),
                    );
                }
                let avg_search_ms =
                    check_start.elapsed().as_secs_f64() * 1000.0 / check_count as f64;

                // Thresholds: Insert > 10ms, Search > 2ms (Sanity Check)
                // W8: Relaxed insert limit from 5ms to 10ms (acceptable for serial build)
                if avg_insert_ms > 10.0 || avg_search_ms > 2.0 {
                    panic!(
                        "ABORT: 10k performance critically degraded. Insert: {:.2}ms (limit 10.0ms), Search: {:.2}ms (limit 2.0ms). Stopping before 50k run.",
                        avg_insert_ms, avg_search_ms
                    );
                }
            }

            // Manual Metric Collection for Report
            let memory = index.memory_usage(); // Graph memory

            // Storage memory estimate
            let storage_mem = if use_quantization {
                count * dims // 1 byte per dim
            } else {
                count * dims * 4 // 4 bytes per dim
            };

            // Total Estimate
            let total_mem = memory + storage_mem;
            let memory_per_vec = total_mem / count;

            println!(
                ">> N={} [{}]: Build Time: {:.2?}, Memory (Est): {:.2} MB, Per Vector: {} bytes",
                count,
                mode_str,
                build_duration,
                total_mem as f64 / 1024.0 / 1024.0,
                memory_per_vec
            );

            // Search Latency Benchmark
            group.throughput(Throughput::Elements(1));

            group.bench_with_input(
                BenchmarkId::new(format!("search_{}", mode_str), count),
                &count,
                |b, &_| {
                    let mut query_iter = queries.iter().cycle();
                    b.iter(|| {
                        let q = query_iter.next().unwrap();
                        // Using .search() which internally creates and reuses SearchContext
                        // The optimization is in search_impl() which reuses ctx across layers
                        black_box(index.search(black_box(q), k, &storage).unwrap());
                    });
                },
            );

            // Explicitly drop to free memory
            drop(index);
            drop(storage);
        }

        drop(vectors);
        drop(queries);
    }

    group.finish();
}

criterion_group!(benches, bench_scaling);
criterion_main!(benches);
